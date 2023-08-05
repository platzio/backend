# Platz Backend

This repo contains Platz's backend. It's written in Rust 🦀 and is broken down to several crates in one workspace:

* `platz-api`
* `platz-db`
* `platz-k8s-agent`
* `platz-chart-discovery`
* `platz-status-updates`

## Running Locally

```bash
docker compose up
```

The `docker-compose.yaml` in this repo will set up a database, OIDC provider and run the API server.

**👉 For a list of users and their passwords see [.dev/oidc-users.json](.dev/oidc-users.json).**

Forwarded ports:

* Port `3000`: API server
* Port `9000`: OIDC provider
* Port `15432`: Postgres database

To connect to the development database:

```bash
PGHOST=127.0.0.1 PGPORT=15432 PGUSER=postgres PGPASSWORD=postgres psql
```

## Developing Against Production Database

> ⚠️ This is potentially dangerous, don't use this method after large or dangerous changes.

Run the following command in a separate tab:

```bash
kubectl --context=control -n platz port-forward platz-postgresql-0 5432:5432
```

Then run any of the workers with the `DATABASE_URL` environment variable set from the directory of the crate you'd like to execute:

```bash
cd api
DATABASE_URL=postgres://postgres:postgres@localhost:5432/platz cargo run -- api
```

## Crates Overview

### `platz-db`

This is a library for accessing the database. Its main goal is to define database migrations and models and, in practice, be the only crate in Platz that depends on `diesel`.

In addition, this crate is responsible for distributing database notifications. This is defined in `events.rs`.

### `platz-api`

The API is a worker that serves the API and handles user authentication.

For authenticating users, it gets the OIDC information from AWS SSM parameters which should be passed as command line arguments.

### `platz-k8s-agent`

This worker tracks Kubernetes clusters, updates their status in the database, and keeps a fresh copy of credentials allowing other parts in the worker to communicate with Kubernetes clusters.

Discovering and communicating with Kubernetes clusters is based on permissions to AWS EKS. This worker automatically discovers EKS clusters from all regions in the same AWS account it's running in.

The first part that needs access to Kubernetes clusters is the `deploy` module. This module watches for pending deployment tasks and runs them one by one.

There are different deployment task types (defined in the `DeploymentTaskOperation` enum), which also act as the history for each deployment:

* **Install**: Creates an initial installation of a deployment. This creates the namespace for the deployment with the correct labels and annotations for Platz to be able to trace it back to its deployment. Once the namespace is created, this task works the same as the **Upgrade** task.
* **Upgrade**: Runs a `helm upgrade` command with the requested Helm chart onto the deployment's namespace.
* **Reinstall**: Same as an **Upgrade** task, but created when a dependent deployment or object has been updated. The main reason this task exists is to contain a reason to be displayed to users.
* **Recreate**: Moves a deployment between namespaces and/or clusters.
* **Uninstall**: Deletes the deployment's namespace.
* **InvokeAction**: Invokes a deployment action, see *Helm Chart Extensions* below.
* **RestartK8sResource**: Restarts a Kubernetes resource, relevant for Kubernetes Deployments and Statefulsets.

The second part is the `k8s/tracker` module. It watches Kubernetes resources and updates their status in the database:

* **Namespaces:** Platz marks each namespace it creates with a `platz=yes` label. This allows it filter and watch for namespace changes. Whenever a namespace is created, updated, or deleted, Platz can mark the appropriate deployment's state. For example, when a deployment is uninstalled, the deployment is marked as `DELETING` and a deployment task is created to delete the deployment namespace. When Platz detects the namespace was deleted, it deletes the deployment object altogether.
* **Kubernetes Deployments** (not to be confused with Platz or Helm deployments, which are different things): Platz tracks and creates/updates Kubernetes deployments in the `k8s_resources` table. This allows displaying deployment status and to restart them.
* **Kubernetes Statefulsets**: Ditto.
* **Kubernetes Jobs**: ditto.

### `platz-chart-discovery`

This worker discovers charts uploaded to ECR: Terraform defines the appropriate AWS resources to receive these notifications, and this worker watches for events in an SQS queue.

See *Helm Chart Extensions* below for more information.

### `platz-status-updates`

This worker is responsible for watching Platz deployments that have enabled the `status` feature in their chart's `features.json`. See *Helm Chart Extensions* below for information on this.

For each deployment, Platz queries the status endpoint and updates the deployment's status in the database. The frontend can then display this information.

## Terraform

Platz has a Terraform part that creates the AWS resources necessary for running some of the workers above.

After making changes, you can apply them by running:

```
./run-terraform.sh
```

with the appropriate AWS credentials (`aws sso login`, define `AWS_PROFILE`, etc.)

The main Terraform files are:

* `api-role.tf` allows `platz-api` to access the OIDC SSM parameters.
* `ecr-events.tf` creates the SQS queue for receiving events on ECR activity.

## Helm Chart Extensions

Platz provides several non-standard extensions to Helm:

### UI Schema

Similar to the standard `values.schema.json` supported by Helm, Platz allowing creating a `values.ui.json` file in the chart's root folder.

This file contains a *UI Schema* which defines arbitrary inputs to be inserted by users. Those inputs are then converted to *outputs* in the chart's values (a.k.a `values.yaml`) and *secrets* to be created directly in the deployment namespace.

### Actions

Based on the UI Schema, actions are defined in `actions.json`.

Each action has its UI Schema for defining its inputs and how to format the request body using its outputs. The secrets are irrelevant in this case.

After successfully creating the request body, it's sent to the *Standard Ingress* endpoint (see below) using the formatted body in JSON format.

### Features

The following features can be enabled in the chart's `features.json` file:

#### Standard Ingress

When set to `true`, Platz will inject an `ingress` section to the chart values.

This `ingress` section has the same format of any ingress section created by a `helm create` command, therefore it's a standard format.

The main components are the domain name, which is generated by Platz since it knows the top-level domain of each cluster and the unique name generated for each deployment.

Also, `ingress` would contain the TLS secret name to allow HTTPS support.

#### Status

Assuming *Standard Ingress* is enabled, a status API path can be provided.

The structs expected to be returned from this endpoint are defined in the `platz-sdk` crate.

When enabled, whenever a deployment runs a chart with this feature, the `platz-status-updates` worker samples the status endpoint every 15s and updates the deployment status in the database.

#### Cardinality

Allows defining whether the deployment can be installed as `OnePerCluster` or `Many`.

When not defined, `Many` is the default.

#### Node Selector and Tolerations

Those are arrays of paths, so arrays of arrays, pointing to additional paths that should be populated with the node selector and tolerations defined in the current env.

This is usually requires when adding a subchart (like a database). Without propagating these values the database pods may run outside the env's node.

Further reading:

* [Assigning Pods to Nodes](https://kubernetes.io/docs/concepts/scheduling-eviction/assign-pod-node/)
* [Taints and Tolerations](https://kubernetes.io/docs/concepts/scheduling-eviction/taint-and-toleration/)

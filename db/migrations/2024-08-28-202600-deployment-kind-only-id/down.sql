drop index "deployment_kinds__name";

-- Handle deployments table
alter table "deployments" add column "kind" varchar;

update deployments
set kind = deployment_kinds.name
from deployment_kinds
where deployment_kinds.id = deployments.kind_id;

-- Handle deployment_permissions table
alter table "deployment_permissions" add column "kind" varchar;

update deployment_permissions
set kind = deployment_kinds.name
from deployment_kinds
where deployment_kinds.id = deployment_permissions.kind_id;

-- Handle deployment_resource_types table
alter table "deployment_resource_types" add column "kind" varchar;

update deployment_resource_types
set deployment_kind = deployment_kinds.name
from deployment_kinds
where deployment_kinds.id = deployment_resource_types.deployment_kind_id;

-- Handle helm_registries table
alter table "helm_registries" add column "kind" varchar;

update helm_registries
set kind = deployment_kinds.name
from deployment_kinds
where deployment_kinds.id = helm_registries.kind_id;

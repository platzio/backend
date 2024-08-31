alter table "deployments"
    alter column "kind_id"
    drop not null;

alter table "deployment_permissions"
    alter column "kind_id"
    drop not null;

alter table "deployment_resource_types"
    alter column "deployment_kind_id"
    drop not null;

alter table "helm_registries"
    alter column "kind_id"
    drop not null;

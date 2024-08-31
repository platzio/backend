alter table "deployments"
    alter column "kind_id"
    set not null;

alter table "deployment_permissions"
    alter column "kind_id"
    set not null;

alter table "deployment_resource_types"
    alter column "deployment_kind_id"
    set not null;

alter table "helm_registries"
    alter column "kind_id"
    set not null;

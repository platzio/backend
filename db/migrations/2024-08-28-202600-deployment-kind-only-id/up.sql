create unique index "deployment_kinds__name" on "deployment_kinds"("name");

alter table "deployments" drop column "kind";
alter table "deployment_permissions" drop column "kind";
alter table "deployment_resource_types" drop column "deployment_kind";
alter table "helm_registries" drop column "kind";

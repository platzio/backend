--- Handle deployments table: Delete kind_id column
alter table deployments
drop column kind_id;

-- Handle deployment_permissions table: Delete kind_id column
alter table deployment_permissions
drop column kind_id;

-- Handle deployment_resource_types table: Delete kind_id column
alter table deployment_resource_types
drop column deployment_kind_id;

-- Handle helm_registries table: Delete kind_id column
alter table helm_registries
drop column kind_id;

-- Drop deployment_kinds
drop table deployment_kinds;

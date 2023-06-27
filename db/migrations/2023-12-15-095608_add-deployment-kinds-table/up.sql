-- Create deployment_kinds
create table deployment_kinds(
  id uuid primary key default uuid_generate_v4(),
  created_at timestamptz not null default now(),
  name varchar not null
);

-- Populate deployment_kinds table according to helm_registries' kind column
insert into deployment_kinds(name, created_at)
select distinct kind, created_at
from helm_registries
order by created_at asc;

-- Handle deployments table: Add kind_id column
alter table deployments
add column kind_id uuid references deployment_kinds(id);

update deployments
set kind_id = deployment_kinds.id
from deployment_kinds
where deployment_kinds.name = deployments.kind;

-- Handle deployment_permissions table: Add kind_id column
alter table deployment_permissions
add column kind_id uuid references deployment_kinds(id);

update deployment_permissions
set kind_id = deployment_kinds.id
from deployment_kinds
where deployment_kinds.name = deployment_permissions.kind;

-- Handle deployment_resource_types table: Add kind_id column
alter table deployment_resource_types
add column deployment_kind_id uuid references deployment_kinds(id);

update deployment_resource_types
set deployment_kind_id = deployment_kinds.id
from deployment_kinds
where deployment_kinds.name = deployment_resource_types.deployment_kind;

-- Handle helm_registries table: Add kind_id column
alter table helm_registries
add column kind_id uuid references deployment_kinds(id);

update helm_registries
set kind_id = deployment_kinds.id
from deployment_kinds
where deployment_kinds.name = helm_registries.kind;

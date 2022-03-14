alter table helm_charts
add column resource_types jsonb default null;

create table deployment_resource_types(
  id uuid primary key default uuid_generate_v4(),
  created_at timestamptz not null default now(),
  env_id uuid references envs(id) on delete cascade,
  deployment_kind varchar not null,
  key varchar not null constraint not_empty check(length(key) > 0),
  spec jsonb not null default '{}'::jsonb
);

create trigger notify_changes after insert or update or delete on deployment_resource_types
for each row execute procedure notify_trigger('id');

create unique index deployment_resource_types__env_id__kind__key
on deployment_resource_types(env_id, deployment_kind, key);

create table deployment_resources(
  id uuid primary key default uuid_generate_v4(),
  created_at timestamptz not null default now(),
  type_id uuid not null references deployment_resource_types(id) on delete cascade,
  deployment_id uuid references deployments(id) on delete set null,
  name varchar not null constraint not_empty check(length(name) > 0),
  exists boolean default true,
  props jsonb not null default '{}'::jsonb,
  sync_status varchar not null default 'Creating',
  sync_reason varchar default null
);

create trigger notify_changes after insert or update or delete on deployment_resources
for each row execute procedure notify_trigger('id');

create unique index deployment_resource_types__env_id__key
on deployment_resources(type_id, name);

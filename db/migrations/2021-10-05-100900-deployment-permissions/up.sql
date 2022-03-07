create table deployment_permissions(
  id uuid primary key default uuid_generate_v4(),
  created_at timestamptz not null default now(),
  env_id uuid not null references envs(id) on delete cascade,
  user_id uuid not null references users(id) on delete cascade,
  kind varchar not null,
  role varchar not null
);

create trigger notify_changes after insert or update or delete on deployment_permissions
for each row execute procedure notify_trigger('id');

create unique index deployment_permissions_env_user_kind
on deployment_permissions(env_id, user_id, kind);

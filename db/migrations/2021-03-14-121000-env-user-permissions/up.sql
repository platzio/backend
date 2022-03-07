create table env_user_permissions(
  id uuid primary key default uuid_generate_v4(),
  created_at timestamptz not null default now(),
  env_id uuid not null references envs(id) on delete cascade,
  user_id uuid not null references users(id) on delete cascade,
  role varchar not null
);

create unique index env_user_role_env_user
on env_user_permissions(env_id, user_id);

create trigger notify_changes after insert or update or delete on env_user_permissions
for each row execute procedure notify_trigger('id');

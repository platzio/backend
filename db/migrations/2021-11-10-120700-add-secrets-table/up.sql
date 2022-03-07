create table secrets (
  id uuid primary key default uuid_generate_v4(),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  env_id uuid not null references envs(id) on delete cascade,
  collection varchar not null,
  name varchar not null,
  contents varchar not null
);

create unique index secrets_env_id_collection_name
on secrets(env_id, collection, name);

create trigger notify_changes after insert or update or delete on secrets
for each row execute procedure notify_trigger('id');

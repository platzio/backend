create table envs(
  id uuid primary key default uuid_generate_v4(),
  created_at timestamptz not null default now(),
  name varchar not null,
  node_selector jsonb not null default '{}'::jsonb,
  tolerations jsonb not null default '{}'::jsonb
);

create trigger notify_changes after insert or update or delete on envs
for each row execute procedure notify_trigger('id');

insert into envs (name) values ('Default');

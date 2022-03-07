create table deployment_tasks (
  id uuid primary key default uuid_generate_v4(),
  created_at timestamptz not null default now(),
  first_attempted_at timestamptz default null,
  started_at timestamptz default null,
  finished_at timestamptz default null,
  deployment_id uuid not null references deployments(id) on delete cascade,
  user_id uuid references users(id) not null,
  operation jsonb not null,
  status varchar not null,
  reason varchar default null
);

create trigger notify_changes after insert or update or delete on deployment_tasks
for each row execute procedure notify_trigger('id');

alter table deployments
add column revision_id uuid references deployment_tasks(id);

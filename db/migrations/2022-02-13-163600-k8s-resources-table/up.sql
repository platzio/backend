create table k8s_resources(
  id uuid primary key default uuid_generate_v4(),
  last_updated_at timestamptz not null default now(),
  deployment_id uuid not null references deployments(id) on delete cascade,
  kind varchar not null,
  api_version varchar not null,
  name varchar not null,
  status_color varchar[] not null,
  metadata jsonb not null
);

create trigger notify_changes after insert or update or delete on k8s_resources
for each row execute procedure notify_trigger('id');

create index k8s_resources__last_updated_at on k8s_resources(last_updated_at);

create table deployments (
  id uuid primary key default uuid_generate_v4(),
  created_at timestamptz not null default now(),
  name varchar not null,
  kind varchar not null,
  cluster_id uuid not null references k8s_clusters(id) on delete cascade,
  enabled boolean default true,
  status varchar default 'Unknown',
  description_md varchar default null,
  reason varchar default null,
  reported_status jsonb default null,
  helm_chart_id uuid references helm_charts(id) on delete cascade,
  config jsonb not null default '{}'::jsonb,
  values_override jsonb not null default '{}'::jsonb
);

create trigger notify_changes after insert or update or delete on deployments
for each row execute procedure notify_trigger('id');

alter table deployments
add constraint deployments_cluster_name_kind_key unique (cluster_id, kind, name);

create table k8s_clusters (
  id uuid primary key default uuid_generate_v4(),
  env_id uuid references envs(id) default null,
  provider_id varchar not null unique,
  created_at timestamptz not null default now(),
  last_seen_at timestamptz not null default now(),
  name varchar not null,
  region_name varchar not null,
  is_ok boolean not null default true,
  not_ok_reason varchar default null,
  ignore boolean default false,
  domain varchar unique default null
);

create trigger notify_changes after insert or update or delete on k8s_clusters
for each row execute procedure notify_trigger('id');

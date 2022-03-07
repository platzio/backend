create table helm_registries (
  id uuid primary key default uuid_generate_v4(),
  created_at timestamptz not null default now(),
  domain_name varchar not null,
  repo_name varchar not null,
  available boolean not null default true,
  fa_icon varchar not null default 'question',
  unique (domain_name, repo_name)
);

create trigger helm_registries_notify after insert or update or delete on helm_registries
for each row execute procedure notify_trigger('id');

create table helm_charts (
  id uuid primary key default uuid_generate_v4(),
  created_at timestamptz not null,
  helm_registry_id uuid not null references helm_registries(id) on delete cascade,
  image_digest varchar not null,
  image_tag varchar not null,
  available boolean not null default true,
  values_ui jsonb default null,
  actions_schema jsonb default null,
  features jsonb default null,
  error varchar default null
);

create trigger notify_changes after insert or update or delete on helm_charts
for each row execute procedure notify_trigger('id');

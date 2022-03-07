create table settings(
  id uuid primary key default uuid_generate_v4(),
  created_at timestamptz not null default now(),
  key varchar unique not null,
  value varchar not null
);

create trigger notify_changes after insert or update or delete on settings
for each row execute procedure notify_trigger('id');

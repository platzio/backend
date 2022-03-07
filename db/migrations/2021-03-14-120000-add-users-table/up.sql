create table users(
  id uuid primary key default uuid_generate_v4(),
  created_at timestamptz not null default now(),
  display_name varchar not null,
  email varchar not null,
  is_admin boolean default false
);

create trigger notify_changes after insert or update or delete on users
for each row execute procedure notify_trigger('id');

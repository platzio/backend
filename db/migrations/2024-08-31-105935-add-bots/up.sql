create table bots(
    id uuid primary key default uuid_generate_v4(),
    created_at timestamptz not null default now(),
    display_name varchar not null
);

create trigger notify_changes after insert or update or delete on "bots"
for each row execute procedure notify_trigger('id');

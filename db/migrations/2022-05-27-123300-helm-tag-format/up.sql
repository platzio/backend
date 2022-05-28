create table helm_tag_formats(
  id uuid primary key default uuid_generate_v4(),
  created_at timestamptz not null default now(),
  pattern varchar unique not null
);

create trigger notify_changes after insert or update or delete on helm_tag_formats
for each row execute procedure notify_trigger('id');

alter table helm_charts
add column tag_format_id uuid default null references helm_tag_formats(id);

alter table helm_charts
add column parsed_version varchar default null;

alter table helm_charts
add column parsed_revision varchar default null;

alter table helm_charts
add column parsed_branch varchar default null;

alter table helm_charts
add column parsed_commit varchar default null;

insert into helm_tag_formats (pattern)
values ('^(chart-)?v?(?P<version>\d+\.\d+\.\d+)((-(?P<revision>\d+))?(-g(?P<commit>[0-9a-zA-Z]+))?-(?P<branch>[-\w]+))?$'),
       ('^(chart-)?v?(?P<version>\d+\.\d+\.\d+)-(?P<branch>[-_A-Za-z0-9]+)\.(?P<revision>\d+)$');

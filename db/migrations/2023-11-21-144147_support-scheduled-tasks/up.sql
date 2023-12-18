alter table deployment_tasks
add column execute_at timestamptz;

update deployment_tasks
set execute_at=created_at;

alter table deployment_tasks
alter column execute_at set default now();

alter table deployment_tasks
alter column execute_at set not null;

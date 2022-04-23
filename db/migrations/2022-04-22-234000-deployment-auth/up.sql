alter table deployment_tasks
rename column user_id to acting_user_id;

alter table deployment_tasks
alter column acting_user_id drop not null;

alter table deployment_tasks
add column acting_deployment_id uuid references deployments(id);

alter table deployment_tasks
add constraint deployment_tasks__acting_user_or_deployment
check ((acting_user_id is null) <> (acting_deployment_id is null));

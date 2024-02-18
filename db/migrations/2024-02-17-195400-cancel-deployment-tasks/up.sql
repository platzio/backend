alter table deployment_tasks
add column canceled_by_user_id uuid references users(id);

alter table deployment_tasks
add column canceled_by_deployment_id uuid references deployments(id);

alter table deployment_tasks
add constraint deployment_tasks__canceled_by_user_or_deployment
check ((canceled_by_user_id is null) or (canceled_by_deployment_id is null));

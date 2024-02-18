alter table deployment_tasks
drop constraint deployment_tasks__canceled_by_user_or_deployment;

alter table deployment_tasks
drop column canceled_by_deployment_id;

alter table deployment_tasks
drop column canceled_by_user_id;

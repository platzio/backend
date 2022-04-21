alter table deployment_tasks
add column cluster_id uuid default null
references k8s_clusters(id);

update deployment_tasks
set cluster_id=(select cluster_id from deployments
                where deployments.id=deployment_tasks.deployment_id);

alter table deployment_tasks
alter column cluster_id drop default;

alter table deployment_tasks
alter column cluster_id set not null;

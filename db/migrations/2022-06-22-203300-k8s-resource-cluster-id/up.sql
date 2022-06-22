delete from k8s_resources;

alter table k8s_resources
add column cluster_id uuid not null
references k8s_clusters(id)
on delete cascade;

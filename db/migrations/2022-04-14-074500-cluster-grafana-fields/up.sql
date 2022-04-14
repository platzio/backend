alter table k8s_clusters
add column grafana_url varchar default null;

alter table k8s_clusters
add column grafana_datasource_name varchar default null;

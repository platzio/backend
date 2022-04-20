alter table k8s_clusters
add column domain_tls_secret_name varchar default null;

update k8s_clusters
set domain_tls_secret_name='tls-wildcard';

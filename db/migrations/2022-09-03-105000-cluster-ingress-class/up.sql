alter table k8s_clusters rename column domain to ingress_domain;
alter table k8s_clusters rename column domain_tls_secret_name to ingress_tls_secret_name;
alter table k8s_clusters add column ingress_class varchar default null;

mod annotations;
mod cluster_type;
mod eks_discovery;
mod pods;
mod tracker;

pub use annotations::{deployment_namespace_annotations, DEPLOYMENT_NAMESPACE_LABELS};
pub use eks_discovery::scan_for_new_clusters;
pub use pods::execute_pod;
pub use tracker::K8S_TRACKER;

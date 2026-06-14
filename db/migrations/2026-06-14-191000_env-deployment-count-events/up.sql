-- Keep per-environment deployment counts live. The env list/detail endpoints
-- return each env's deployment count, and the frontend keeps it current by
-- refetching an env whenever it receives an `envs` change event. A deployment
-- insert/delete (or a move between clusters) changes a count but does not touch
-- the envs table, so this trigger emits a synthetic `envs` refresh event on the
-- generic `db_notifications` channel for the affected environment(s).
--
-- Only count-affecting changes emit, to avoid noise from frequent status/config
-- updates: INSERT (new env), DELETE (old env), and UPDATE only when the
-- deployment moved to a different cluster (and thus possibly a different env).

CREATE FUNCTION notify_env_deployment_count() RETURNS trigger AS $trigger$
DECLARE
  new_env UUID;
  old_env UUID;
BEGIN
  IF TG_OP = 'INSERT' THEN
     SELECT env_id INTO new_env FROM k8s_clusters WHERE id = NEW.cluster_id;
  ELSIF TG_OP = 'DELETE' THEN
     SELECT env_id INTO old_env FROM k8s_clusters WHERE id = OLD.cluster_id;
  ELSIF TG_OP = 'UPDATE' AND NEW.cluster_id IS DISTINCT FROM OLD.cluster_id THEN
     SELECT env_id INTO new_env FROM k8s_clusters WHERE id = NEW.cluster_id;
     SELECT env_id INTO old_env FROM k8s_clusters WHERE id = OLD.cluster_id;
  ELSE
     RETURN NULL;
  END IF;

  IF new_env IS NOT NULL THEN
     PERFORM pg_notify(
       'db_notifications',
       '{"timestamp":"' || CURRENT_TIMESTAMP
         || '","operation":"UPDATE","schema":"public","table":"envs","env_id":"'
         || new_env::TEXT || '","data":{"id":"' || new_env::TEXT || '"}}');
  END IF;

  IF old_env IS NOT NULL AND old_env IS DISTINCT FROM new_env THEN
     PERFORM pg_notify(
       'db_notifications',
       '{"timestamp":"' || CURRENT_TIMESTAMP
         || '","operation":"UPDATE","schema":"public","table":"envs","env_id":"'
         || old_env::TEXT || '","data":{"id":"' || old_env::TEXT || '"}}');
  END IF;

  RETURN NULL;
END;
$trigger$ LANGUAGE plpgsql;

CREATE TRIGGER notify_env_deployment_count_changes
AFTER INSERT OR UPDATE OR DELETE ON deployments
FOR EACH ROW EXECUTE PROCEDURE notify_env_deployment_count();

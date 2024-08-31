create trigger notify_changes after insert or update or delete on "deployment_kinds"
for each row execute procedure notify_trigger('id');

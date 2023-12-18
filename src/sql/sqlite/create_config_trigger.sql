CREATE TRIGGER update_config_updated_at AFTER UPDATE ON invman_config
       BEGIN
            UPDATE invman_config SET updated_at=(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')) WHERE id=new.id;
       END;

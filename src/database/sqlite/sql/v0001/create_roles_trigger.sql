CREATE TRIGGER update_roles_updated_at AFTER UPDATE ON invman_roles
       BEGIN
            UPDATE invman_roles SET updated_at=(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')) WHERE id=new.id;
       END;

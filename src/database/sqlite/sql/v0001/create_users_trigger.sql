CREATE TRIGGER update_users_updated_at AFTER UPDATE ON invman_users
       BEGIN
            UPDATE invman_users SET updated_at=(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')) WHERE id=new.id;
       END;

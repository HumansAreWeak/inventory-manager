CREATE TRIGGER update_articles_updated_at AFTER UPDATE ON invman_inventory
       BEGIN
            UPDATE invman_inventory SET updated_at=(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')) WHERE id=new.id;
       END;

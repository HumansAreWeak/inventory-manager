CREATE TRIGGER after_user_registration AFTER INSERT ON invman_users
       BEGIN
            INSERT INTO invman_event_tx (action_no, dispatcher) VALUES (100, new.id);
       END;

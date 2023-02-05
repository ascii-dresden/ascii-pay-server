INSERT INTO account 
    (balance_cents, balance_coffee_stamps, balance_bottle_stamps, name, email, role)
  VALUES
    (473, 11, 5, 'John Doe', 'john.doe@example.com', 'member'),
    (0, 0, 0, 'Poor Guy', 'poor.guy@example.org', 'basic'),
    (81, 1, 4, 'Mr Root', 'root@localhost', 'admin');

INSERT INTO account_auth_method
    (account_id, login_key, data)
  VALUES
    (1, 'user-johndoe', '{"Password":{"username":"johndoe","password_hash":[32,32,32,32,32,32,32,32,32,32,32,32,32,32,32,32,32,32,32,32,32,32,32,32,32,32,32,32,32,32,32,32]}}');
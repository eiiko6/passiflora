-- Users
INSERT INTO users (username, password_hash, email) VALUES
('alice', '$argon2id$v=19$m=19456,t=2,p=1$W0OzC/dmZQt7/xUJt4E9hA$cYiUC91a5yCQU9tDUadw0FKjUmTRv453cYwu1nfMKUQ', 'alice@example.com'),
('bob', '$argon2id$v=19$m=19456,t=2,p=1$1T7VaQps1X5Wj+TJHt8FIQ$/hA7PSITskjELwfNw+s6BvCJmUA4dDDrSGJvDvHx7Kc', 'bob@example.com');

-- Items for users
INSERT INTO items (user_id, name, value) VALUES
(1, 'Alice Item 1', 'Hello world 1'),
(1, 'Alice Item 2', 'Lorem ipsum'),
(2, 'Bob Item 1', 'HELLO WORLD 2');


INSERT INTO accounts(id,username,password_hash)
SELECT :account_id,:username,:password_hash
WHERE NOT EXISTS (SELECT 1 FROM account_roles WHERE role_code='super_admin');
INSERT INTO account_roles(account_id,role_code)
SELECT id,'super_admin' FROM accounts WHERE id=:account_id AND username=:username
AND NOT EXISTS (SELECT 1 FROM account_roles WHERE role_code='super_admin')
ON CONFLICT DO NOTHING;

test_db_up:
	createuser sqlm-test
	createdb -O sqlm-test sqlm-test
	psql -U sqlm-test sqlm-test < testdb.sql

test_db_down:
	dropdb sqlm-test
	dropuser sqlm-test

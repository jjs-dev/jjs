import os
import pymongo
import urllib.parse


def db_connect_url(db_url: str) -> pymongo.database.Database:
    """
    This function connects to MongoDB database using provided URL

    URL must look like mongodb://[used:password@]host:port/database_name
    all except database_name is forwarded to mongodb client, database_name
    denotes concrete database on mongodb cluster.
    Port usually is 27017.

    Example for mongodb instance, running on localhost without authentication:
    mongodb://localhost:27017/jjs
    Of course, any other database name can be used.
    """

    # strip is used, because urllib returns path in form `/jjs`,
    # so we want to strip leading '/'
    db_name = urllib.parse.urlparse(db_url).path[1:]

    client = pymongo.MongoClient(db_url)

    return client[db_name]


def db_connect_via_env() -> pymongo.database.Database:
    """
    Connects to MongoDB database using URL in DATABASE_URL
    environment variable. See `db_connect_url` function for url format.
    """
    db_url = os.environ["DATABASE_URL"]
    return db_connect_url(db_url)

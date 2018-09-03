import pika
import os
import shutil

PATH = os.path.dirname(os.path.realpath(__file__))

JJS_ROOT = '/var/jjs'
TEST_DATA_ROOT = f"{PATH}/test_data"
for item in os.listdir(JJS_ROOT):
    shutil.rmtree(f"{JJS_ROOT}/{item}")

for item in os.listdir(TEST_DATA_ROOT):
    shutil.copytree(f"{TEST_DATA_ROOT}/{item}", f"{JJS_ROOT}/{item}")

connection = pika.BlockingConnection(pika.ConnectionParameters('localhost'))
channel = connection.channel()

QUEUE_NAME = 'jjs_invoker'

channel.queue_declare(queue=QUEUE_NAME, durable=True)
channel.queue_purge(queue=QUEUE_NAME)
TC_PATH = f"{PATH}/test_commands"
# print("writing from", TC_PATH)

items = os.listdir(TC_PATH)
items.sort()
for item in items:
    channel.basic_publish(exchange='', routing_key=QUEUE_NAME, body=open(f"{TC_PATH}/{item}").read())
connection.close()

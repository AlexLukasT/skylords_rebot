import requests
import time

BASE_URL = "http://127.0.0.1:7273"

with open("requests/hello.json", "r") as f:
    requests.post(BASE_URL + "/hello", data=f.read())

time.sleep(0.5)

with open("requests/prepare.json", "r") as f:
    requests.post(BASE_URL + "/prepare", data=f.read())

time.sleep(0.5)

with open("requests/start.json", "r") as f:
    requests.post(BASE_URL + "/start", data=f.read())
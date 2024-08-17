import json
import matplotlib.pyplot as plt

with open("locations.json", "r") as f:
    locations = json.loads(f.read())

x_list = []
y_list = []
colors = []
for loc_name, loc in locations.items():
    if "token" in loc:
        x_list.append(loc["token"]["x"])
        y_list.append(loc["token"]["y"])
        colors.append("red")
        plt.text(loc["token"]["x"], loc["token"]["y"], loc_name)
    else:
        plt.text(loc["powers"]["1"]["x"], loc["powers"]["1"]["y"], loc_name)
    for i, power in loc["powers"].items():
        x_list.append(power["x"])
        y_list.append(power["y"])
        colors.append("blue")
        plt.text(power["x"], power["y"], str(i))

plt.scatter(x_list, y_list, c=colors)
plt.show()
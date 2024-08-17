import json

locations = {}
with open("locations.txt", "r") as f:
    for line in f.readlines():
        des, vals = line.strip().split(" ")
        x_part, y_part = vals.split(",")
        x = float(x_part.split(":")[1])
        y = float(y_part.split(":")[1][:-1])
        values = {"x":x, "y":y}
        des = des[:-1]
        des_parts = des.split(".")
        loc = des_parts[0]
        if loc not in locations:
            locations[loc] = {}
        if len(des_parts) == 2:
            locations[loc]["token"] = values
        else:
            index = int(des_parts[2])
            if not "powers" in locations[loc]:
                locations[loc]["powers"] = {}
            locations[loc]["powers"][index] = values

print(json.dumps(locations, indent=2))

with open("locations.json", "w") as f:
    f.write(json.dumps(locations, indent=2))
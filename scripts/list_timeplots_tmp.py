import glob, os
tp = os.path.join(os.environ["USERPROFILE"], "AppData", "LocalLow",
                  "Unicorn One", "AIComp", "Saves", "Tennis", "Timeplots")
fs = sorted(glob.glob(os.path.join(tp, "*.json")), key=os.path.getmtime)
print("count:", len(fs))
for f in fs[-8:]:
    print(os.path.basename(f), os.path.getsize(f))

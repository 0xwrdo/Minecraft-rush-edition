import os
import urllib.request
import zipfile

CLIENT_URL = "https://piston-data.mojang.com/v1/objects/e877b6a07acd633fb3bb475002175cec036e7b87/client.jar"
TEMP_JAR = os.path.join(os.environ.get("TEMP", "."), "mc_client.jar")
BASE_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "assets"))
GUI_DIR = os.path.join(BASE_DIR, "modern_gui")
ITEM_DIR = os.path.join(BASE_DIR, "modern_items")

os.makedirs(GUI_DIR, exist_ok=True)
os.makedirs(ITEM_DIR, exist_ok=True)

if not os.path.exists(TEMP_JAR):
    print("Downloading client.jar from official Mojang manifest...")
    urllib.request.urlretrieve(CLIENT_URL, TEMP_JAR)
    print("Download complete.")

print("Extracting GUI and Item textures...")
with zipfile.ZipFile(TEMP_JAR, 'r') as z:
    gui_count = 0
    item_count = 0
    for name in z.namelist():
        if name.startswith("assets/minecraft/textures/gui/") and name.endswith(".png"):
            rel_path = name[len("assets/minecraft/textures/gui/"):]
            dest_path = os.path.join(GUI_DIR, rel_path)
            os.makedirs(os.path.dirname(dest_path), exist_ok=True)
            with open(dest_path, "wb") as f:
                f.write(z.read(name))
            gui_count += 1
        elif name.startswith("assets/minecraft/textures/item/") and name.endswith(".png"):
            rel_path = name[len("assets/minecraft/textures/item/"):]
            dest_path = os.path.join(ITEM_DIR, rel_path)
            os.makedirs(os.path.dirname(dest_path), exist_ok=True)
            with open(dest_path, "wb") as f:
                f.write(z.read(name))
            item_count += 1

print(f"Extraction complete! GUI textures: {gui_count}, Item textures: {item_count}")


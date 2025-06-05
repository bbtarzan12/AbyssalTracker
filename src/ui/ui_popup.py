import tkinter as tk
from tkinter import ttk
from tkinter import simpledialog
import os
from datetime import datetime
from ..logic.price import AbyssalDataManager # AbyssalDataManager 재사용

class AbyssalResultPopup:
    ABYSSAL_TYPES = [
        "T1 Exotic", "T1 Firestorm", "T1 Gamma", "T1 Dark", "T1 Electrical",
        "T2 Exotic", "T2 Firestorm", "T2 Gamma", "T2 Dark", "T2 Electrical",
        "T3 Exotic", "T3 Firestorm", "T3 Gamma", "T3 Dark", "T3 Electrical",
        "T4 Exotic", "T4 Firestorm", "T4 Gamma", "T4 Dark", "T4 Electrical",
        "T5 Exotic", "T5 Firestorm", "T5 Gamma", "T5 Dark", "T5 Electrical",
        "T6 Exotic", "T6 Firestorm", "T6 Gamma", "T6 Dark", "T6 Electrical",
    ]

    def __init__(self, data_manager: AbyssalDataManager):
        self.data_manager = data_manager
        self.result = {}
        self.root = None # This will be the Toplevel window for the popup
        self.parent_root = None # This will be the main Tkinter root
        self.abyssal_type_var = None
        self.item_text = None

    def set_parent_root(self, parent_root):
        self.parent_root = parent_root

    def show_popup(self):
        if not self.parent_root:
            print("[ERROR] Parent Tkinter root not set for popup.")
            return {}

        self.root = tk.Toplevel(self.parent_root)
        self.root.title("Abyssal Run Result")
        self.root.geometry("400x350")
        self.root.resizable(False, False)
        self.root.attributes('-topmost', True)
        self.root.protocol("WM_DELETE_WINDOW", self._on_ok) # Handle window close button

        tk.Label(self.root, text="어비셜 종류:").pack(anchor="w", padx=10, pady=(10,0))
        self.abyssal_type_var = tk.StringVar(value=self.ABYSSAL_TYPES[0])
        type_combo = ttk.Combobox(self.root, textvariable=self.abyssal_type_var, values=self.ABYSSAL_TYPES, state="readonly")
        type_combo.pack(fill="x", padx=10, pady=5)

        tk.Label(self.root, text="획득 아이템 (붙여넣기):").pack(anchor="w", padx=10, pady=(10,0))
        self.item_text = tk.Text(self.root, height=10, width=40)
        self.item_text.pack(fill="both", padx=10, pady=5, expand=True)

        ok_btn = tk.Button(self.root, text="확인", command=self._on_ok)
        ok_btn.pack(pady=10)

        # Instead of mainloop, we wait for the window to close
        self.parent_root.wait_window(self.root)
        return self.result

    def _on_ok(self):
        self.result["abyssal_type"] = self.abyssal_type_var.get()
        self.result["items"] = self.item_text.get("1.0", tk.END).strip()
        self.root.destroy()

    def save_result(self, start_time, end_time):
        self.data_manager.save_abyssal_result(self.result, start_time, end_time)

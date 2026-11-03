# -*- coding: utf-8 -*-
"""
Convierte un workflow de ComfyUI guardado por la interfaz (formato UI, que puede
traer subgrafos) al formato API que espera POST /prompt.

Por qué existe: los workflows que se descargan de comfy.org vienen en formato UI
con subgrafos. ComfyUI NO acepta eso en /prompt; la conversión la hace el
navegador, no el servidor. Este script la hace aquí, contra el /object_info real
del ComfyUI que esté corriendo, sin inventarse nombres de entradas.

Uso:
    python tools/ui_a_api.py entrada.json salida.json [--base http://127.0.0.1:8188]

Solo lee el JSON de entrada y escribe el de salida. No toca ComfyUI salvo para
preguntar /object_info (lectura).
"""
from __future__ import annotations

import json
import sys
import urllib.request

BASE = "http://127.0.0.1:8188"

_cache: dict[str, dict] = {}
_omitidos: list[str] = []


def objeto(clase: str) -> dict | None:
    """Esquema real del nodo según el ComfyUI que esté corriendo (o None)."""
    if clase in _cache:
        return _cache[clase]
    try:
        with urllib.request.urlopen(f"{BASE}/object_info/{clase}", timeout=90) as r:
            d = json.load(r)
    except Exception:
        d = {}
    _cache[clase] = d.get(clase)
    return _cache[clase]


def campos_widget(clase: str) -> list[tuple[str, bool, bool]]:
    """[(nombre, es_widget, control_after_generate)] en el orden en que la UI
    guarda `widgets_values`."""
    n = objeto(clase)
    if not n:
        return []
    orden = n.get("input_order", {})
    entrada = n.get("input", {})
    salida = []
    for seccion in ("required", "optional"):
        for nombre in orden.get(seccion, []):
            spec = entrada.get(seccion, {}).get(nombre)
            if not spec:
                continue
            t = spec[0]
            es_widget = (
                isinstance(t, list)
                or isinstance(t, dict)
                or not isinstance(t, str)
                or t.split(",")[0].strip()
                in ("INT", "FLOAT", "STRING", "BOOLEAN", "COMBO", "COMBO")
            )
            extra = spec[1] if len(spec) > 1 and isinstance(spec[1], dict) else {}
            salida.append((nombre, es_widget, bool(extra.get("control_after_generate"))))
    return salida


def indice_links(links) -> dict:
    """id -> (origin_id, origin_slot, target_id, target_slot)."""
    m = {}
    for l in links or []:
        if isinstance(l, dict):
            m[l["id"]] = (l["origin_id"], l["origin_slot"], l["target_id"], l["target_slot"])
        elif isinstance(l, (list, tuple)) and len(l) >= 4:
            m[l[0]] = (l[1], l[2], l[3], l[4] if len(l) > 4 else None)
    return m


def convertir(wf: dict) -> dict:
    """Devuelve el workflow en formato API {id: {class_type, inputs}}."""
    api: dict[str, dict] = {}
    omitidos: list[str] = []
    global _omitidos
    _omitidos = omitidos

    def nodo_api(nid, nodo, ref, prefijo, enlaces, proxy_origen=None, proxy_destino=None):
        clase = nodo.get("type")
        if not clase or nodo.get("mode") == 4:  # nodo silenciado: no se ejecuta
            return
        esquema = objeto(clase)
        if esquema is None:
            # Nodos de interfaz (MarkdownNote, Note, Reroute...) no existen en el
            # servidor. Si alguna salida suya se usaba, /prompt lo dirá.
            omitidos.append(f"{nid}:{clase}")
            return

        entradas: dict[str, object] = {}
        valores = nodo.get("widgets_values")
        lista = valores if isinstance(valores, list) else []
        dictval = valores if isinstance(valores, dict) else {}
        idx = 0

        # 1) valores de widgets, en el orden del esquema
        for nombre, es_widget, ctrl in campos_widget(clase):
            if not es_widget:
                continue
            if isinstance(valores, dict):
                if nombre in dictval:
                    entradas[nombre] = dictval[nombre]
                continue
            if idx < len(lista):
                entradas[nombre] = lista[idx]
            idx += 1
            if ctrl and idx < len(lista):
                idx += 1  # el valor de control (fixed/randomize) no va al API

        # 2) enlaces: mandan sobre los widgets
        for e in nodo.get("inputs") or []:
            lid = e.get("link")
            if lid is None:
                continue
            if lid in enlaces:
                o, oslot, _, _ = enlaces[lid]
                if proxy_origen is not None and o == proxy_origen:
                    continue  # viene de una entrada promocionada; se resuelve fuera
                if o in ref:
                    entradas[e["name"]] = [f"{prefijo}{o}", oslot]
            elif proxy_origen is not None:
                entradas[e["name"]] = None  # lo rellenará la promoción

        api[f"{prefijo}{nid}"] = {"class_type": clase, "inputs": entradas}

    # ---------- nivel raíz ----------
    raiz = {n["id"]: n for n in wf.get("nodes") or []}
    enlaces_raiz = indice_links(wf.get("links"))
    subgrafos = {s["id"]: s for s in (wf.get("definitions") or {}).get("subgraphs") or []}

    for nid, nodo in raiz.items():
        clase = nodo.get("type")
        if clase in subgrafos:
            continue  # se expande abajo
        nodo_api(nid, nodo, raiz, "", enlaces_raiz)

    # ---------- subgrafos ----------
    for nid, inst in raiz.items():
        sg = subgrafos.get(inst.get("type"))
        if not sg:
            continue
        pref = f"s{nid}_"
        enl_sg = indice_links(sg.get("links"))
        nodos_sg = {n["id"]: n for n in sg.get("nodes") or []}
        p_in = (sg.get("inputNode") or {}).get("id")
        p_out = (sg.get("outputNode") or {}).get("id")

        # Qué entrada interna (nombre de entrada del nodo destino) se alimenta
        # desde el exterior: los links que salen del proxy de entrada -> nodo.
        promovidas: dict[str, object] = {}
        entradas_inst = inst.get("inputs") or []
        for i, entrada_sg in enumerate(sg.get("inputs") or []):
            if i >= len(entradas_inst):
                break
            lid_ext = entradas_inst[i].get("link")
            origen = None
            if lid_ext is not None and lid_ext in enlaces_raiz:
                o, oslot, _, _ = enlaces_raiz[lid_ext]
                if o in raiz:
                    origen = [str(o), oslot]
            for lid_int in entrada_sg.get("linkIds") or []:
                if lid_int in enl_sg:
                    _, _, t, tslot = enl_sg[lid_int]
                    if t in nodos_sg:
                        nombre = None
                        for e in nodos_sg[t].get("inputs") or []:
                            if e.get("link") == lid_int:
                                nombre = e["name"]
                                break
                        if nombre:
                            promovidas[f"{t}.{nombre}"] = origen
        # los widgets del subgrafo (p. ej. resolución) también pueden venir de fuera
        # y se quedan con su valor por defecto del nodo interno.

        for nid_i, nodo_i in nodos_sg.items():
            if nodo_i.get("mode") == 4:
                continue
            nodo_api(nid_i, nodo_i, nodos_sg, pref, enl_sg, proxy_origen=p_in)
            clave = f"{pref}{nid_i}"
            for e in nodo_i.get("inputs") or []:
                if e.get("link") is None:
                    continue
                k = f"{nid_i}.{e['name']}"
                if k in promovidas:
                    if promovidas[k] is None:
                        api[clave]["inputs"].pop(e["name"], None)
                    else:
                        api[clave]["inputs"][e["name"]] = promovidas[k]

        # Salidas del subgrafo -> consumidores de fuera
        salidas_sg = sg.get("outputs") or []
        salidas_inst = inst.get("outputs") or []
        for j, salida_sg in enumerate(salidas_sg):
            if j >= len(salidas_inst):
                break
            productor = None
            for lid_int in salida_sg.get("linkIds") or []:
                if lid_int in enl_sg:
                    o, oslot, t, _ = enl_sg[lid_int]
                    if t == p_out and o in nodos_sg:
                        productor = [f"{pref}{o}", oslot]
                        break
            if productor is None:
                continue
            for lid_ext in salidas_inst[j].get("links") or []:
                if lid_ext not in enlaces_raiz:
                    continue
                _, _, t_ext, tslot = enlaces_raiz[lid_ext]
                if t_ext not in raiz:
                    continue
                destino = raiz[t_ext]
                if destino.get("type") in subgrafos:
                    continue
                clave = f"{t_ext}"
                if clave not in api:
                    continue
                for e in destino.get("inputs") or []:
                    if e.get("link") == lid_ext:
                        api[clave]["inputs"][e["name"]] = productor
                        break

    if omitidos:
        print("nodos de interfaz omitidos:", ", ".join(omitidos))
    return api


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        print(__doc__)
        return 2
    entrada, salida = argv[1], argv[2]
    if "--base" in argv:
        global BASE
        BASE = argv[argv.index("--base") + 1]
    with open(entrada, encoding="utf-8-sig") as f:
        wf = json.load(f)
    if "nodes" not in wf:
        print("Esto ya parece formato API; no hay nada que convertir.")
        return 1
    api = convertir(wf)
    with open(salida, "w", encoding="utf-8") as f:
        json.dump(api, f, indent=1, ensure_ascii=False)
    print(f"{len(api)} nodos API escritos en {salida}")
    for k, v in api.items():
        print(f"  {k}: {v['class_type']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))

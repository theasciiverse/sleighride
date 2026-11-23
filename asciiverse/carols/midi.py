import struct, sys
def vlq(b, i):
    v = 0
    while True:
        c = b[i]; i += 1; v = (v << 7) | (c & 0x7f)
        if not c & 0x80: return v, i
def read(path):
    b = open(path, 'rb').read()
    assert b[:4] == b'MThd'
    fmt, ntr, div = struct.unpack('>HHH', b[8:14])
    i = 14; tracks = []; meta = {'tempo': [], 'timesig': [], 'key': []}
    for _ in range(ntr):
        assert b[i:i+4] == b'MTrk'; ln = struct.unpack('>I', b[i+4:i+8])[0]; j = i + 8; end = j + ln; i = end
        t = 0; run = None; on = {}; notes = []; name = ''
        while j < end:
            d, j = vlq(b, j); t += d
            st = b[j]
            if st == 0xff:
                typ = b[j+1]; ln2, k = vlq(b, j+2); data = b[k:k+ln2]; j = k + ln2
                if typ == 0x51: meta['tempo'].append((t, int.from_bytes(data, 'big')))
                elif typ == 0x58: meta['timesig'].append((t, data[0], 2 ** data[1]))
                elif typ == 0x59: meta['key'].append((t, int.from_bytes(data[:1], 'big', signed=True), data[1]))
                elif typ in (3, 4): name = data.decode('latin-1')
                continue
            if st in (0xf0, 0xf7):
                ln2, k = vlq(b, j+1); j = k + ln2; continue
            if st & 0x80: run = st; j += 1
            kind = run & 0xf0
            if kind in (0x80, 0x90):
                n, v = b[j], b[j+1]; j += 2
                ch = run & 0x0f
                if kind == 0x90 and v > 0: on[(ch, n)] = t
                elif (ch, n) in on:
                    s = on.pop((ch, n)); notes.append((s, t - s, n, ch))
            elif kind in (0xc0, 0xd0): j += 1
            else: j += 2
        tracks.append({'name': name, 'notes': sorted(notes)})
    return div, meta, tracks
if __name__ == '__main__':
    for p in sys.argv[1:]:
        div, meta, tracks = read(p)
        print(f"== {p}: div {div}, time {[(a,b) for _,a,b in meta['timesig']][:2]}, key {[(k,m) for _,k,m in meta['key']][:1]}, tempo {[round(60e6/x) for _,x in meta['tempo']][:2]}")
        for i, tr in enumerate(tracks):
            if tr['notes']:
                ns = [n for _,_,n,_ in tr['notes']]
                beats = (tr['notes'][-1][0] + tr['notes'][-1][1]) / div
                print(f"   track {i} {tr['name'][:20]!r:22} {len(ns):4} notes, range {min(ns)}-{max(ns)}, {beats:.0f} beats")

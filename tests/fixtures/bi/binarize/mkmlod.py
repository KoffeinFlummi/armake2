import struct, sys
def cs(s): return s.encode()+b'\0'
def lod(faces_tex, res=1.0):
    pts=[(0,0,0),(1,0,0),(0,1,0),(2,0,0),(3,0,0),(2,1,0)]
    b=b'P3DM'+struct.pack('<IIIIII',0x1c,0x100,len(pts),len(faces_tex),len(faces_tex),0)
    for p in pts: b+=struct.pack('<fffI',*p,0)
    for _ in faces_tex: b+=struct.pack('<fff',0,0,-1)
    for i,(tex,mat) in enumerate(faces_tex):
        b+=struct.pack('<I',3)
        for k,(u,v) in enumerate([(0,0),(1,0),(0,1)]): b+=struct.pack('<IIff',3*i+k,i,u,v)
        b+=struct.pack('<IIff',0,0,0,0)+struct.pack('<I',0)+cs(tex)+cs(mat)
    b+=b'TAGG'+b'\x01'+cs('#EndOfFile#')+struct.pack('<I',0)+struct.pack('<f',res)
    return b
open(sys.argv[1],'wb').write(b'MLOD'+struct.pack('<II',0x101,1)+lod([('t21\\data\\glass_ca.paa',''),('t21\\data\\solid_co.paa','')]))

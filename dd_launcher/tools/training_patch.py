# Run with the game's armv7 slice extracted to /home/claude/re/v7 (not distributed).
# Prints nothing useful without it; kept so the patch can be rebuilt and checked.
# Builds the --patch-u32 list for Training mode (armv7 slice of ddiPhone).
import struct, keystone
d = bytearray(open('/home/claude/re/v7','rb').read())
orig = bytes(d)
ks = keystone.Ks(keystone.KS_ARCH_ARM, keystone.KS_MODE_THUMB)
def asm(src, addr):
    enc, _ = ks.asm(src, addr); return bytes(enc)
def put(addr, b):
    d[addr-0x1000:addr-0x1000+len(b)] = b

T = 150  # enemies walk back only when farther than this from the nearest player

# A: DamageCalcSub returns 0 damage when the target is an enemy (r0 < 0 here).
CA = 0x1e00
put(CA, asm("""
  cmp r0, #0
  ite lt
  movlt r0, #0
  movge r0, lr
  pop {pc}
""", CA))
put(0xd934, asm(f"b.w {CA:#x}", 0xd934))

# B: ObjDamageCalc does nothing to enemies.
CB = 0x1e20
put(CB, asm(f"""
  ldrsb.w r3, [r2, #0x16d]
  cmp r3, #0
  it lt
  bxlt lr
  ldr.w r3, [r2, #0x134]
  b.w {0xd940:#x}
""", CB))
put(0xd93c, asm(f"b.w {CB:#x}", 0xd93c))

# C: enemy AI. Keys were already cleared; only run the AI (movement only)
# when the enemy is off screen (camera left edge at +0x47a0, 480 wide) or far
# from every live player, so it walks back into view and then stands still.
CC = 0x1e40
put(CC, asm(f"""
  push {{r4, r5, r6, r7, r8, lr}}
  mov r6, r0
  mov r7, r2
  ldr.w r3, [r4, #0x5c]
  movw r0, #0xffff
  movt r0, #0x7fff
  movw r12, #0x2550
  ldr.w r1, [r5, r12]
  cbz r1, p2
  ldr r2, [r1, #0x48]
  tst.w r2, #0x8000
  bne p2
  ldr r2, [r1, #0x5c]
  subs r2, r2, r3
  it lt
  neglt r2, r2
  cmp r2, r0
  it lt
  movlt r0, r2
p2:
  movw r12, #0x2554
  ldr.w r1, [r5, r12]
  cbz r1, chk
  ldr r2, [r1, #0x48]
  tst.w r2, #0x8000
  bne chk
  ldr r2, [r1, #0x5c]
  subs r2, r2, r3
  it lt
  neglt r2, r2
  cmp r2, r0
  it lt
  movlt r0, r2
chk:
  cmp r0, #{T}
  bgt run
  movw r12, #0x47a0
  ldr.w r1, [r5, r12]
  subs r2, r3, r1
  cmp r2, #20
  blt run
  cmp.w r2, #460
  ble done
run:
  mov r0, r6
  mov r1, r4
  blx r7
  ldr.w r2, [r4, #0xd4]
  and r2, r2, #0xf
  str.w r2, [r4, #0xd4]
  ldr.w r2, [r4, #0xd8]
  and r2, r2, #0xf
  str.w r2, [r4, #0xd8]
  ldr.w r2, [r4, #0xdc]
  and r2, r2, #0xf
  str.w r2, [r4, #0xdc]
  ldr.w r2, [r4, #0xe0]
  and r2, r2, #0xf
  str.w r2, [r4, #0xe0]
done:
  pop {{r4, r5, r6, r7, r8, pc}}
""", CC))
put(0x16c6e, asm(f"bl {CC:#x}", 0x16c6e))

# D: freeze the stage timer (skip the countdown).
put(0x12952, asm("nop", 0x12952))

out = []
for off in range(0, len(d), 4):
    a, b = orig[off:off+4], d[off:off+4]
    if a != b:
        va = off + 0x1000
        out.append(f"--patch-u32={va:#x},{struct.unpack('<I',a)[0]:#x},{struct.unpack('<I',b)[0]:#x}")
open('/home/claude/re/training_patch.txt','w').write(' '.join(out))
open('/home/claude/re/v7_training','wb').write(d)
print(len(out), 'words'); print(' '.join(out)[:400])

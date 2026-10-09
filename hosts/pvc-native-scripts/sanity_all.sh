#!/bin/bash
# dev-native hook sanity (NOT the official measurement; dev-measure owns those). SHARED CPU lock only (exclusive lock starved behind other Developers; numbers are hook-sanity, not measurements).
cd /workspace/dev_work/dev-native; L=/workspace/orchestrator_work/pm/CPU_LOCK
for r in gl cpu; do
 for s in idle checkbox toggle-show tab type-latin; do
  flock -s $L scripts/run_native.sh $s --renderer $r --out run/sanity/$r/$s > /dev/null 2>&1; echo "$r $s rc=$?"
 done
 PTR_IN=connect_button PTR_OUT=bio_text flock -s $L scripts/run_native.sh ptr-move --renderer $r --artifact /workspace/dev_work/dev-ir/specimens/L01.pvca --out run/sanity/$r/ptr-move-L01 >/dev/null 2>&1; echo "$r ptr-move-L01 rc=$?"
done
echo ALLDONE

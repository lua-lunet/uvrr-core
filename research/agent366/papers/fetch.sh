#!/bin/zsh
# agent366 reference harvest — downloads every external work cited in docs/* or
# paper/* (plus Raft, per user instruction) into research/agent366/papers/.
# Append-only log of what was fetched lives in ../log.md.
set -u
cd "$(dirname "$0")"

fetch() { # $1 = output name, $2 = url
  if [ -s "$1" ]; then echo "SKIP $1 (exists)"; return 0; fi
  echo "GET  $1 <- $2"
  curl -sSL --fail --retry 2 --connect-timeout 20 -o "$1" "$2" \
    && echo "OK   $1 ($(stat -f%z "$1") bytes)" \
    || { echo "FAIL $1"; rm -f "$1"; }
}

fetch vr-revisited-2012.pdf          https://pmg.csail.mit.edu/papers/vr-revisited.pdf
fetch paxos-made-simple-2001.pdf     https://lamport.azurewebsites.net/pubs/paxos-simple.pdf
fetch vertical-paxos-2009.pdf        https://lamport.azurewebsites.net/pubs/vertical-paxos.pdf
fetch smaug-2017-gorke-armknecht.pdf  https://arxiv.org/pdf/1708.04871
fetch shraer2012-atc12-dynamic-reconfig.pdf https://www.usenix.org/system/files/conference/atc12/atc12-final74.pdf
fetch diskless-disc2017-recovering-shared-objects.pdf "https://drops.dagstuhl.de/storage/00lipics/lipics-vol091-disc2017/LIPIcs.DISC.2017.36/LIPIcs.DISC.2017.36.pdf"
fetch lamport1998-part-time-parliament.pdf https://lamport.azurewebsites.net/pubs/lamport-paxos.pdf
fetch lamport2004-cheap-paxos.pdf            https://lamport.azurewebsites.net/pubs/web-dsn-submission.pdf
fetch lamport2008-stoppable-paxos.pdf        https://lamport.azurewebsites.net/pubs/stoppable.pdf
fetch birman2010-virtually-synchronous.pdf  https://www.microsoft.com/en-us/research/wp-content/uploads/2010/11/vs-submit.pdf
fetch lorch2006-smart.pdf                   https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/eurosys2006.pdf
fetch burrows2006-chubby-osdi06.pdf          https://www.usenix.org/legacy/events/osdi06/tech/full_papers/burrows/burrows.pdf
fetch reconfiguring-a-state-machine-2010.pdf "http://www-sop.inria.fr/members/Francesco.Bongiovanni/reconfiguring%20a%20state%20machine.pdf"
fetch osdi14-pillai-allfs.pdf        https://www.usenix.org/system/files/conference/osdi14/osdi14-paper-pillai.pdf
fetch sosp13-optimistic-crash.pdf    https://research.cs.wisc.edu/adsl/Publications/optfs-sosp13.pdf
fetch lampson-sturgis-1979.pdf       "https://www.microsoft.com/en-us/research/wp-content/uploads/1979/01/21.-Lampson-and-Sturgis-Crash-recovery-in-a-distributed-data-storage-system.pdf"
fetch fast18-alagappan-par.pdf       https://www.usenix.org/system/files/conference/fast18/fast18-alagappan.pdf
fetch diskless-tr16.pdf              https://syslab.cs.washington.edu/papers/diskless-tr16.pdf
fetch disk-paxos-2003.pdf            https://lamport.azurewebsites.net/pubs/disk-paxos-disc.pdf
fetch corfu-nsdi12.pdf               https://www.usenix.org/system/files/conference/nsdi12/nsdi12-final75.pdf
fetch zookeeper-atc10-hunt.pdf       https://www.usenix.org/events/usenix10/tech/full_papers/Hunt.pdf
fetch vr-1988-oki-liskov.pdf         https://www.cs.princeton.edu/courses/archive/fall11/cos518/papers/viewstamped.pdf
fetch oki-dissertation-tr423.pdf     https://publications.csail.mit.edu/lcs/pubs/pdf/MIT-LCS-TR-423.pdf
fetch fqi-2016-howard.pdf            https://arxiv.org/pdf/1608.06696
fetch raft-atc14-ongaro.pdf          https://www.usenix.org/system/files/conference/atc14/atc14-paper-ongaro.pdf
fetch lean4-cade28.pdf               https://leanprover.github.io/papers/lean4.pdf
fetch turner-paxos-reconf.tex        "https://raw.githubusercontent.com/DaveCTurner/paxos-membership/raft-like-reconfiguration/paxos-reconf.tex"
echo DONE

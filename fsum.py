#!/usr/bin/env python2

import os, sys
from stat import S_ISDIR, S_ISLNK

def fsum(args):
  args = args[:]                        # we modify args in-place
  seen = {}
  size = 0
  for fl in args:
    try:
      st = os.lstat(fl)
      if S_ISLNK(st.st_mode):
        if os.path.exists(fl):
          st = os.stat(fl)
        else:
          continue                      # don't carp on dangling symlinks
      if seen.has_key((st.st_dev, st.st_ino)):
        continue
      if S_ISDIR(st.st_mode):
        args.extend([os.path.join(fl, x) for x in os.listdir(fl)])
      else:
        size += st.st_size
      seen[(st.st_dev, st.st_ino)] = 1
    except OSError, reason:
      print >>sys.stderr, reason
  return size

if __name__ == '__main__':
  size = fsum(sys.argv[1:])
  print size
  for power, digits, letter in (1<<10, 0, 'K'), (1<<20, 2, 'M'), (1<<30, 2, 'G'), (1<<40, 2, 'T'):
    if size >= power:
      print "%.*f %c" % (digits, size / float(power), letter)

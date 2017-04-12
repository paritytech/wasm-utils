int ops = 0;

void log(int block_ops) {
    ops += block_ops;
}

int call(int x) {
  log(5);
  return 2 * x;
}
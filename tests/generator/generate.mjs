/** Prints a mixed voucher sheet size for operators. Logic lives in testdata.rs. */
const n = Number(process.argv[2] || 1000);
console.log(JSON.stringify({ vouchers: n, edge_every: 23, payment_blocks: 5 }));

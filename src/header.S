    .section .text
    .global _trap
_trap:
    bra     _trap

_irq:
    rte


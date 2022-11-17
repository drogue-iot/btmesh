MEMORY
{
  /* NOTE 1 K = 1 KiBi = 1024 bytes */
  FLASH                             : ORIGIN = 0x00027000, LENGTH = 356352
  STORAGE                           : ORIGIN = 0x0007E000, LENGTH = 4K
  STORAGE_EXTRA                     : ORIGIN = 0x0007F000, LENGTH = 4K
  RAM                               : ORIGIN = 0x2000c698, LENGTH = 120440
}

__storage = ORIGIN(STORAGE);
__storage_extra = ORIGIN(STORAGE_EXTRA);

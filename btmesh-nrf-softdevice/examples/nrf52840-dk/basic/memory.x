MEMORY
{
  /* NOTE 1 K = 1 KiBi = 1024 bytes */
  /* These values correspond to the NRF52840 with Softdevices S140 7.0.1 */

  FLASH : ORIGIN = 0x00027000, LENGTH = 864K
  STORAGE : ORIGIN = 0x000FF000, LENGTH = 4K
  RAM : ORIGIN = 0x2000CD28, LENGTH = 204K 


}

__storage = ORIGIN(STORAGE);


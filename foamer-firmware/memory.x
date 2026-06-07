PAGE_SIZE = 4K;
PAGE_COUNT = 512;

MEMORY {
    BOOT2 : ORIGIN = 0x10000000, LENGTH = 0x100
    FLASH : ORIGIN = 0x10000100, LENGTH = 2048K - 0x100 - PAGE_SIZE
    PERSONALIZATION : ORIGIN = 0x10000000 + (PAGE_SIZE * (PAGE_COUNT - 1)), LENGTH = PAGE_SIZE

    /* Pick one of the two options for RAM layout     */

    /* OPTION A: Use all RAM banks as one big block   */
    /* Reasonable, unless you are doing something     */
    /* really particular with DMA or other concurrent */
    /* access that would benefit from striping        */
    RAM   : ORIGIN = 0x20000000, LENGTH = 264K

    /* OPTION B: Keep the unstriped sections separate */
    /* RAM: ORIGIN = 0x20000000, LENGTH = 256K        */
    /* SCRATCH_A: ORIGIN = 0x20040000, LENGTH = 4K    */
    /* SCRATCH_B: ORIGIN = 0x20041000, LENGTH = 4K    */
}

SECTIONS {
    .flash_pointers ORIGIN(BOOT2) : {
        PROVIDE(FLASH_START = .);
    } >BOOT2

    .personalization_sector ORIGIN(PERSONALIZATION) (NOLOAD) : ALIGN(8) {
        . = ALIGN(8);
        PROVIDE(PERSONALIZATION_SECTOR = .);
        . += PAGE_SIZE;
        . = ALIGN(8);
    } >PERSONALIZATION
}

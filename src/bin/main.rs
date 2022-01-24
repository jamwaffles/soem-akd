const EC_TIMEOUTRXM: i32 = 70_000;

fn akd_setup(slave: u16, context: soem::Context) -> anyhow::Result<()> {
    log::debug!("Setup AKD, PO2SO hook");

    // Clear SM PDO
    context.write_sdo(slave, 0x1C12, 00, 0x00, EC_TIMEOUTRXM)?;
    // Clear SM PDO
    context.write_sdo(slave, 0x1C13, 00, 0x00, EC_TIMEOUTRXM)?;

    // CSP Fixed PDO
    // context.write_sdo(slave, 0x1C12, 01, 0x1701, EC_TIMEOUTRXM)?;
    // Fixed PDO, allows CSP target position
    // context.write_sdo(slave, 0x1C12, 01, 0x1724, EC_TIMEOUTRXM)?;
    // Synchronous velocity mode
    context.write_sdo(slave, 0x1C12, 01, 0x1702, EC_TIMEOUTRXM)?;

    // One item mapped
    context.write_sdo(slave, 0x1C12, 00, 0x01, EC_TIMEOUTRXM)?;
    // Read position from PL.FB instead of FB1.P
    // context.write_sdo(slave, 0x1C13, 01, 0x1b24, EC_TIMEOUTRXM)?;
    // Set fixed TXPDO
    context.write_sdo(slave, 0x1C13, 01, 0x1B01, EC_TIMEOUTRXM)?;
    // One item mapped
    context.write_sdo(slave, 0x1C13, 00, 0x01, EC_TIMEOUTRXM)?;
    // Opmode - Cyclic Synchronous Position
    // context.write_sdo(slave, 0x6060, 00, 0x08;, EC_TIMEOUTRXM)?;
    context.write_sdo(slave, 0x6060, 00, 0x09, EC_TIMEOUTRXM)?;

    // Interpolation time period
    context.write_sdo(slave, 0x60C2, 01, 0x02, EC_TIMEOUTRXM)?;
    // Interpolation time index
    context.write_sdo(slave, 0x60C2, 02, 0xfd, EC_TIMEOUTRXM)?;

    // Scale based on 0x6091 and 0x6092 https://www.kollmorgen.com/en-us/developer-network/position-scaling-akd-drive-ethercat-communication/
    // FBUS.PARAM05
    // context.write_sdo(slave, 0x36E9, 00, 0b10000, EC_TIMEOUTRXM)?;
    // FBUS.PARAM05
    context.write_sdo(slave, 0x36E9, 00, 0x00, EC_TIMEOUTRXM)?;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    Ok(())
}

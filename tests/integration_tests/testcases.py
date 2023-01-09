from waveform_tools import COBC, CEP

def wait_for_heartbeats(cobc: COBC, n=5, timeout=0.2):
    toggle = 'high'
    for _ in range(n):
        cobc.heartbeat_pin.wait_for(toggle, timeout)
        if toggle == 'high':
            toggle = 'low'
        else:
            toggle = 'high'
    print(f"Received {n} heartbeats")

def store_archive(cobc: COBC):
    print("Storing Archive")
    file = open("./student_program.zip", "rb").read()
    cobc.uart.send(CEP.with_data(b'\x01\x00\x00'))
    cobc.uart.expect(CEP.ACK, 1)
    cobc.uart.send(CEP.with_data(file))
    cobc.uart.expect(CEP.ACK, 1)
    cobc.uart.send(CEP.EOF)
    cobc.uart.expect(CEP.ACK, 1)
    

def test_get_status_none(cobc: COBC):
    wait_for_heartbeats(cobc)
    print("Sending Get Status")
    cobc.uart.send(CEP.with_data(b'\x04'))
    data = cobc.uart.receive(9, 2)
    packets = CEP.parse_multiple_packets(data)
    print(f"Received {packets}")
    assert packets[0] == CEP.ACK
    assert packets[1] == CEP.with_data(b'\x00')
    cobc.uart.send(CEP.ACK)

def test_store_execute_and_return(cobc: COBC):
    wait_for_heartbeats(cobc)
    cobc.update_pin.expect_to_be('low')
    store_archive(cobc)
    print("Executing Program")
    cobc.uart.send(CEP.with_data(b'\x02\x00\x00\x03\x00\x02\x00'))
    cobc.uart.expect([CEP.ACK, CEP.ACK], 1)
    cobc.update_pin.wait_for('high', 1)
    print("Update Pin is high")
    cobc.uart.send(CEP.with_data(b'\x04'))
    cobc.uart.expect([CEP.ACK, CEP.with_data(b'\x01\x00\x00\x03\x00\x00')], 1)
    cobc.uart.send(CEP.ACK)
    cobc.update_pin.expect_to_be('high')
    print("Got exit status")
    cobc.uart.send(CEP.with_data(b'\x04'))
    cobc.uart.expect([CEP.ACK, CEP.with_data(b'\x02\x00\x00\x03\x00')], 1)
    cobc.uart.send(CEP.ACK)
    print("Got Result ready")
    cobc.uart.send(CEP.with_data(b'\x05'))
    cobc.uart.send(CEP.ACK)
    cobc.uart.send(CEP.ACK)
    cobc.update_pin.wait_for('low', 1)
    print("Received result")
    
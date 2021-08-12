from terra_sdk.client.lcd import LCDClient, Wallet
from terra_sdk.key.mnemonic import MnemonicKey

from terra_sdk.util.contract import read_file_as_b64, get_code_id, get_contract_address
from terra_sdk.core.auth import StdFee
from terra_sdk.core.wasm import MsgStoreCode, MsgInstantiateContract, MsgExecuteContract

import pathlib
import sys
sys.path.append(pathlib.Path(__file__).parent.resolve())

client = LCDClient(url="https://tequila-lcd.terra.dev", chain_id="tequila-0004")
mnemonic = "napkin guess language merit split slice source happy field search because volcano staff section depth clay inherit result assist rubber list tilt chef start"
deployer = Wallet(lcd=client, key=MnemonicKey(mnemonic))
std_fee = StdFee(4000000, "1000000uusd")


def send_msg(msg):
    tx = deployer.create_and_sign_tx(
        msgs=[msg], fee=std_fee
    )
    return client.tx.broadcast(tx)

def store_contract(contract_name: str) -> str:
    bytes = read_file_as_b64(f"artifacts/{contract_name}.wasm")
    msg = MsgStoreCode(deployer.key.acc_address, bytes)
    result = send_msg(msg)
    return get_code_id(result)

def instantiate_contract(code_id: str, init_msg) -> str:
    msg = MsgInstantiateContract(
        owner=deployer.key.acc_address,
        code_id=code_id,
        init_msg=init_msg
    )
    result = send_msg(msg)
    print(result)
    return get_contract_address(result)

def execute_contract(contract_addr: str, execute_msg):
    msg = MsgExecuteContract(
        sender=deployer.key.acc_address, 
        contract=contract_addr, 
        execute_msg=execute_msg
    )
    return send_msg(msg)


print("Excecuting Burn")
contract_address = 'terra1uf3qnx93g5ddt4l6hnratmcjwuwt84w79w0vwm'
print(f'On contract:  {contract_address}')
result = execute_contract(contract_addr=contract_address, execute_msg={
    "send_to_burn_account": {}
})

# print("ExecuteMsg response")
# print(result)


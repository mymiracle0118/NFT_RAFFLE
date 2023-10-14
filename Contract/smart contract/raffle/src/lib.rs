use borsh::{BorshDeserialize,BorshSerialize};
use anchor_lang::{prelude::*, Discriminator, AnchorDeserialize, AnchorSerialize, Key, solana_program::{sysvar::{clock::Clock}, program::{invoke}}};
use arrayref::{array_ref};
use anchor_spl::token::{self, Token, TokenAccount, Mint, Transfer};
use std::cell::Ref;

declare_id!("rafrZNbxGdfFUBzddkzgtcHLqijmjarEihYcUuCuByV");

#[program]
pub mod raffle{
    use super::*;

    pub fn init_raffle_system(
        ctx : Context<InitRaffleSystem>,
        _bump : u8
        ) -> ProgramResult {
        let raffle_system = &mut ctx.accounts.raffle_system;
        raffle_system.owner = ctx.accounts.owner.key();
        raffle_system.manager = ctx.accounts.manager.key();
        raffle_system.rand = *ctx.accounts.rand.key;
        raffle_system.token_mint = ctx.accounts.token_mint.key();
        raffle_system.token_account = ctx.accounts.token_account.key();
        raffle_system.pause_flag = false;
        raffle_system.bump = _bump;
        Ok(())
    }

    pub fn transfer_authority(
        ctx : Context<TransferAuthority>,
        _new_owner : Pubkey,
        )->ProgramResult{
        let raffle_system = &mut ctx.accounts.raffle_system;
        raffle_system.owner = _new_owner;
        Ok(())
    }

    pub fn set_manager(
        ctx : Context<SetManager>,
        _new_owner : Pubkey,
        )->ProgramResult{
        let raffle_system = &mut ctx.accounts.raffle_system;
        raffle_system.manager = _new_owner;
        Ok(())
    }

    pub fn init_raffle(
        ctx : Context<InitRaffle>,
        _room_name : String,
        _logo : String,
        _discord : String,
        _twitter : String,
        _ticket_value : u64,
        _spot_num : u32,
        _max_ticket_num : u32,
        _max_ticket_per_user : u32,
        ) -> ProgramResult {
        let raffle = &mut ctx.accounts.raffle;
        raffle.raffle_system = ctx.accounts.raffle_system.key();
        raffle.room_name = _room_name;
        raffle.logo = _logo;
        raffle.discord = _discord;
        raffle.twitter = _twitter;
        raffle.ticket_value = _ticket_value;
        raffle.spot_num = _spot_num;
        raffle.max_ticket_num = _max_ticket_num;
        raffle.ledger_account = *ctx.accounts.ledger.key;
        raffle.spots_account = *ctx.accounts.spot_store.key;
        raffle.is_show = true;
        raffle.max_ticket_per_user = _max_ticket_per_user;

        let mut ledger_data = (&mut ctx.accounts.ledger).data.borrow_mut();
        let mut new_data = Ledger::discriminator().try_to_vec().unwrap();
        new_data.append(&mut raffle.key().try_to_vec().unwrap());
        new_data.append(&mut (0 as u32).try_to_vec().unwrap());
        for i in 0..new_data.len(){
            ledger_data[i] = new_data[i];
        }

        let mut spots_data = (&mut ctx.accounts.spot_store).data.borrow_mut();
        let mut spots_new_data: Vec<u8> = SpotStore::discriminator().try_to_vec().unwrap();
        spots_new_data.append(&mut raffle.key().try_to_vec().unwrap());
        spots_new_data.append(&mut _spot_num.try_to_vec().unwrap());
        for i in 0..spots_new_data.len(){
            spots_data[i] = spots_new_data[i];
        }

        Ok(())
    }

    pub fn init_user_data(
        ctx : Context<InitUserData>,
        _bump : u8
        ) -> ProgramResult {
        let user_data =  &mut ctx.accounts.user_data;
        user_data.owner = ctx.accounts.owner.key();
        user_data.raffle = ctx.accounts.raffle.key();
        user_data.ticket_num = 0;
        user_data.bump = _bump;
        Ok(())
    }

    pub fn update_raffle(
        ctx : Context<UpdateRaffle>,
        _room_name : String,
        _logo : String,
        _discord : String,
        _twitter : String,
        ) -> ProgramResult {
        let raffle = &mut ctx.accounts.raffle;
        raffle.room_name = _room_name;
        raffle.logo = _logo;
        raffle.discord = _discord;
        raffle.twitter = _twitter;
        Ok(())
    }

    pub fn put_spot(
        ctx : Context<PutSpot>,
        _index : u32,
        ) -> ProgramResult {
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info().clone(),
            Transfer{
                from : ctx.accounts.nft_from.to_account_info().clone(),
                to : ctx.accounts.nft_to.to_account_info().clone(),
                authority : ctx.accounts.owner.to_account_info().clone()    
            }
        );
        token::transfer(cpi_ctx, 1)?;
        set_spot(&mut ctx.accounts.spot_store, _index as usize, Spot{
            nft : ctx.accounts.nft.key(),
            winner_ticket : 0,
            claimed : false,
        });
        Ok(())
    }

    pub fn redeem_spot(
        ctx : Context<RedeemSpot>,
        _index : u32,
        ) -> ProgramResult {
        let spot = get_spot(&ctx.accounts.spot_store, _index as usize)?;
        if spot.nft != ctx.accounts.nft.key(){
            return Err(PoolError::NotMatch.into());
        }
        let raffle_system = &mut ctx.accounts.raffle_system;
        let raffle_system_seeds = &[raffle_system.rand.as_ref(),&[raffle_system.bump]];
        let signer = &[&raffle_system_seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info().clone(),
            Transfer{
                from : ctx.accounts.nft_from.to_account_info().clone(),
                to : ctx.accounts.nft_to.to_account_info().clone(),
                authority : raffle_system.to_account_info().clone()
            },
            signer
        );
        token::transfer(cpi_ctx, 1)?;
        set_spot(&mut ctx.accounts.spot_store, _index as usize, Spot{
            nft : Pubkey::default(),
            winner_ticket : 0,
            claimed : false,
        });
        Ok(())
    }

    pub fn start_raffle(
        ctx : Context<StartRaffle>,
        _period : u64,
        ) -> ProgramResult {
        let raffle = &mut ctx.accounts.raffle;
        if raffle.status != 0 {
            return Err(PoolError::InvalidStatus.into());
        }
        raffle.start_time = (Clock::from_account_info(&ctx.accounts.clock)?).unix_timestamp as u64;
        raffle.period = _period;
        raffle.status = 1;
        Ok(())
    }

    pub fn buy_ticket(
        ctx : Context<BuyTicket>,
        _num : u32,
        _value : u64
        ) -> ProgramResult {

        let raffle = &mut ctx.accounts.raffle;
        let raffle_system = &mut ctx.accounts.raffle_system;
        let user_data = &mut ctx.accounts.user_data;
        if raffle.max_ticket_per_user!=0 {
            if user_data.ticket_num > raffle.max_ticket_per_user{
                msg!("Wallet limit error");
                return Err(PoolError::AlreadyOverflowTicketNum.into());
            }
            if user_data.ticket_num + _num > raffle.max_ticket_per_user{
                msg!("Wallet limit error");
                return Err(PoolError::OverflowTicketNumPerUser.into());
            }  
        }
        let current_user_num = get_num(&ctx.accounts.ledger.data.borrow())?;
        if raffle.status != 1{
            msg!("Invalid status");
            return Err(PoolError::InvalidStatus.into());
        }
        let clock = (Clock::from_account_info(&ctx.accounts.clock)?).unix_timestamp as u64;
        if clock > raffle.start_time + raffle.period{
            return Err(PoolError::TimeOut.into());
        }
        if (current_user_num + _num as usize) > raffle.max_ticket_num as usize{
            return Err(PoolError::Overflow.into());
        }

        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info().clone(),
            Transfer{
                from : ctx.accounts.token_from.to_account_info().clone(),
                to : ctx.accounts.token_to.to_account_info().clone(),
                authority : ctx.accounts.owner.to_account_info().clone()    
            }
        );

        token::transfer(cpi_ctx, raffle.ticket_value * _num as u64)?;

        if raffle_system.pause_flag {
            sol_transfer_to_pool(
                SolTransferToPoolParams{
                    source : ctx.accounts.owner.to_account_info().clone(),
                    destination : ctx.accounts.raffle_system.clone(),
                    system : ctx.accounts.system_program.to_account_info().clone(),
                    amount : _value
                }
            )?;
        }

        set_user(&mut ctx.accounts.ledger, current_user_num as usize, ctx.accounts.owner.key(), _num);
        set_count(&mut ctx.accounts.ledger, current_user_num as u32 + _num);

        user_data.ticket_num += _num;
        Ok(())
    }

    pub fn end_raffle(
        ctx : Context<EndRaffle>
        ) -> ProgramResult {
        let raffle = &mut ctx.accounts.raffle;
        let ticket_num = get_num(&ctx.accounts.ledger.data.borrow())?;
        let clock = (Clock::from_account_info(&ctx.accounts.clock)?).unix_timestamp as u64;
        if ticket_num!=0{
            let mut winners : Vec<u32> = vec![];
            for i in 0..raffle.spot_num{
                let rand = ((clock+i as u64)*(i as u64 + 1)) % ticket_num as u64;
                let spot = get_spot(&ctx.accounts.spot_store, i as usize)?;
                if spot.winner_ticket != 0 {
                    winners.push(spot.winner_ticket as u32);
                } else {
                    winners.push(rand as u32);
                }
            }
            set_winner(&mut ctx.accounts.spot_store, winners);
            raffle.status = 2;
        }else{
            raffle.status = 0;
        }
        Ok(())
    }

    pub fn end_state(
        ctx : Context<EndState>,
        _ticket_count : u64
        ) -> ProgramResult {
        let raffle = &mut ctx.accounts.raffle;
        let ticket_num = get_num(&ctx.accounts.ledger.data.borrow())?;
        if ticket_num!=0{
            let mut winners : Vec<u32> = vec![];
            for _i in 0..raffle.spot_num{
                let rand = _ticket_count;
                winners.push(rand as u32);
            }
            set_winner(&mut ctx.accounts.spot_store, winners);
        }
        
        Ok(())
    }

    pub fn show_raffle(
        ctx : Context<ShowRaffle>,
        _is_show : bool,
        ) -> ProgramResult {
        let raffle = &mut ctx.accounts.raffle;
        raffle.is_show = _is_show;
        Ok(())
    }

    pub fn set_pause(
        ctx : Context<Pause>,
        _flag : bool
    ) -> ProgramResult {

        let raffle_system = &mut ctx.accounts.raffle_system;

        if raffle_system.manager != *ctx.accounts.owner.key {
            msg!("Invalid Owner");
            return Err(PoolError::InvalidPoolOwner.into());
        }

        raffle_system.pause_flag = _flag;

        Ok(())
    }

    pub fn claim(
        ctx : Context<Claim>,
        _amount : u64
    ) -> ProgramResult {

        let raffle_system = &mut ctx.accounts.raffle_system;
        if raffle_system.manager != *ctx.accounts.owner.key {
            msg!("Invalid manager");
            return Err(PoolError::InvalidPoolOwner.into());
        }

        sol_transfer(
            &mut ctx.accounts.raffle_system_address,
            &mut ctx.accounts.owner,
            _amount
        )?;

        // raffle_system.pause_flag = false;

        Ok(())
    }

    pub fn claim_nft(
        ctx : Context<ClaimNft>,
        num : u32,
        ) -> ProgramResult {
        let raffle_system = &mut ctx.accounts.raffle_system;
        let raffle = &ctx.accounts.raffle;
        if raffle.status != 2 {
            return Err(PoolError::InvalidStatus.into());
        }
        let spot = get_spot(&ctx.accounts.spot_store, num as usize)?;
        if spot.claimed {
            return Err(PoolError::AlreadyClaimed.into());
        }
        let wallet = get_user(&ctx.accounts.ledger, spot.winner_ticket as usize)?;
        if wallet != ctx.accounts.owner.key(){
            return Err(PoolError::NotMatch.into());
        }

        let raffle_system_seeds = &[raffle_system.rand.as_ref(),&[raffle_system.bump]];
        let signer = &[&raffle_system_seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info().clone(),
            Transfer{
                from : ctx.accounts.nft_from.to_account_info().clone(),
                to : ctx.accounts.nft_to.to_account_info().clone(),
                authority : raffle_system.to_account_info().clone()
            },
            signer
        );
        token::transfer(cpi_ctx, 1)?;

        set_spot(&mut ctx.accounts.spot_store, num as usize, Spot{
            nft : spot.nft,
            winner_ticket : spot.winner_ticket,
            claimed : true,
        });

        Ok(())
    }

    pub fn redeem_token(
        ctx : Context<RedeemToken>,
        amount : u64,
        )->ProgramResult{
        let raffle_system = &mut ctx.accounts.raffle_system;
        let raffle_system_seeds = &[raffle_system.rand.as_ref(),&[raffle_system.bump]];
        let signer = &[&raffle_system_seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info().clone(),
            Transfer{
                from : ctx.accounts.token_from.to_account_info().clone(),
                to : ctx.accounts.token_to.to_account_info().clone(),
                authority : raffle_system.to_account_info().clone(),
            },
            signer
        );
        token::transfer(cpi_ctx, amount)?;
        Ok(())
    }

}

struct SolTransferToPoolParams<'a> {
    /// CHECK:
    pub source: AccountInfo<'a>,
    /// CHECK:
    pub destination: ProgramAccount<'a, RaffleSystem>,
    /// CHECK:
    pub system: AccountInfo<'a>,
    /// CHECK:
    pub amount: u64,
}

fn sol_transfer_to_pool(params: SolTransferToPoolParams<'_>) -> ProgramResult {
    let SolTransferToPoolParams {
        source,
        destination,
        system,
        amount
    } = params;

    let result = invoke(
        &anchor_lang::solana_program::system_instruction::transfer(
            source.key,
            &destination.key(),
            amount,
        ),
        &[source, destination.to_account_info(), system],
    );

    result.map_err(|_| PoolError::SolTransferFailed.into())
}

fn sol_transfer(
    from_account: &AccountInfo,
    to_account: &AccountInfo,
    amount_of_lamports: u64,
) -> ProgramResult {
    // Does the from account have enough lamports to transfer?
    if **from_account.try_borrow_lamports()? < amount_of_lamports {
        msg!("Insufficent funds");
        return Err(PoolError::InsufficentFunds.into());
    }
    // Debit from_account and credit to_account
    **from_account.try_borrow_mut_lamports()? -= amount_of_lamports;
    **to_account.try_borrow_mut_lamports()? += amount_of_lamports;
    Ok(())
}

#[derive(Accounts)]
pub struct RedeemToken<'info>{
    #[account(mut)]
    owner : Signer<'info>,

    #[account(mut, has_one=owner)]
    raffle_system : ProgramAccount<'info,RaffleSystem>,

    #[account(mut, address=raffle_system.token_account)]
    token_from : Account<'info, TokenAccount>,

    #[account(mut)]
    token_to : Account<'info, TokenAccount>,

    token_program : Program<'info, Token>   
}


#[derive(Accounts)]
pub struct ClaimNft<'info>{
    #[account(mut)]
    owner : Signer<'info>,

    #[account(mut)]
    raffle_system : ProgramAccount<'info, RaffleSystem>,

    #[account(mut)]
    raffle : ProgramAccount<'info, Raffle>,

    #[account(mut)]
    spot_store : AccountInfo<'info>,

    ledger : AccountInfo<'info>,

    #[account(mut)]
    nft_from : Account<'info, TokenAccount>,

    #[account(mut)]
    nft_to : Account<'info, TokenAccount>,

    token_program : Program<'info, Token>
}

#[derive(Accounts)]
pub struct EndRaffle<'info>{
    #[account(mut)]
    owner : Signer<'info>,

    #[account(has_one=owner)]
    raffle_system : ProgramAccount<'info, RaffleSystem>,

    #[account(mut, has_one=raffle_system)]
    raffle : ProgramAccount<'info, Raffle>,

    #[account(mut)]
    spot_store : AccountInfo<'info>,

    ledger : AccountInfo<'info>,

    clock : AccountInfo<'info>
}

#[derive(Accounts)]
pub struct EndState<'info>{
    #[account(mut)]
    owner : Signer<'info>,

    #[account(mut, constraint= raffle_system.manager==owner.key())]
    raffle_system : ProgramAccount<'info, RaffleSystem>,

    #[account(mut, has_one=raffle_system)]
    raffle : ProgramAccount<'info, Raffle>,

    #[account(mut)]
    spot_store : AccountInfo<'info>,

    ledger : AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct BuyTicket<'info>{
    #[account(mut)]
    owner : Signer<'info>,

    #[account(mut)]
    raffle_system : ProgramAccount<'info, RaffleSystem>,

    #[account(mut)]
    raffle : ProgramAccount<'info, Raffle>,

    #[account(mut, constraint= user_data.owner==owner.key() && user_data.raffle==raffle.key())]
    user_data : ProgramAccount<'info, UserData>,

    #[account(mut)]
    ledger : AccountInfo<'info>,

    #[account(mut, constraint= token_from.owner==owner.key() && token_from.mint==raffle_system.token_mint)]
    token_from : Account<'info, TokenAccount>,

    #[account(mut, address=raffle_system.token_account)]
    token_to : Account<'info, TokenAccount>,

    token_program : Program<'info, Token>,

    system_program : Program<'info, System>,

    clock : AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct ShowRaffle<'info>{
    #[account(mut)]
    owner : Signer<'info>,

    #[account(has_one=owner)]
    raffle_system : ProgramAccount<'info, RaffleSystem>,

    #[account(mut, has_one=raffle_system)]
    raffle : ProgramAccount<'info, Raffle>,
}

#[derive(Accounts)]
pub struct UpdateRaffle<'info>{
    #[account(mut)]
    owner : Signer<'info>,

    #[account(has_one=owner)]
    raffle_system : ProgramAccount<'info, RaffleSystem>,

    #[account(mut, has_one=raffle_system)]
    raffle : ProgramAccount<'info, Raffle>,
}

#[derive(Accounts)]
pub struct StartRaffle<'info>{
    #[account(mut)]
    owner : Signer<'info>,

    #[account(has_one=owner)]
    raffle_system : ProgramAccount<'info, RaffleSystem>,

    #[account(mut, has_one=raffle_system)]
    raffle : ProgramAccount<'info, Raffle>,

    clock : AccountInfo<'info>
}

#[derive(Accounts)]
#[instruction(_index : u32)]
pub struct RedeemSpot<'info>{
    #[account(mut)]
    owner : Signer<'info>,

    #[account(has_one=owner)]
    raffle_system : ProgramAccount<'info, RaffleSystem>,

    #[account(mut,
        has_one=raffle_system,
        constraint= raffle.status==0
            && raffle.spots_account==(*spot_store.key)
            && raffle.spot_num>_index)]
    raffle : ProgramAccount<'info, Raffle>,

    #[account(mut)]
    spot_store : AccountInfo<'info>,

    #[account(mut)]
    nft : Account<'info, Mint>,

    #[account(mut, constraint= nft_from.owner==raffle_system.key() && nft_from.mint==nft.key())]
    nft_from : Account<'info, TokenAccount>,

    #[account(mut, constraint= nft_to.owner==owner.key() && nft_to.mint==nft.key())]
    nft_to : Account<'info, TokenAccount>,    

    token_program : Program<'info, Token>
}

#[derive(Accounts)]
#[instruction(_index : u32)]
pub struct PutSpot<'info>{
    #[account(mut)]
    owner : Signer<'info>,

    #[account(has_one=owner)]
    raffle_system : ProgramAccount<'info, RaffleSystem>,

    #[account(mut,
        has_one=raffle_system,
        constraint= raffle.status==0
            && raffle.spots_account==(*spot_store.key)
            && raffle.spot_num>_index)]
    raffle : ProgramAccount<'info, Raffle>,

    #[account(mut)]
    spot_store : AccountInfo<'info>,

    #[account(mut)]
    nft : Account<'info, Mint>,

    #[account(mut, constraint= nft_from.owner==owner.key() && nft_from.mint==nft.key())]
    nft_from : Account<'info, TokenAccount>,

    #[account(mut, constraint= nft_to.owner==raffle_system.key() && nft_to.mint==nft.key())]
    nft_to : Account<'info, TokenAccount>,

    token_program : Program<'info, Token>
}

#[derive(Accounts)]
#[instruction(_bump : u8)]
pub struct InitUserData<'info>{
    #[account(mut)]
    owner : Signer<'info>,

    raffle : ProgramAccount<'info, Raffle>,

    #[account(init, payer=owner, space=8+USERDATA_SIZE, seeds=[owner.key().as_ref(), raffle.key().as_ref()], bump=_bump)]
    user_data : ProgramAccount<'info, UserData>,

    system_program : Program<'info, System>
}

#[derive(Accounts)]
pub struct InitRaffle<'info>{
    #[account(mut)]
    owner : Signer<'info>,
    
    #[account(has_one=owner)]
    raffle_system : ProgramAccount<'info, RaffleSystem>,
    
    #[account(init, payer=owner, space=8+RAFFLE_SIZE)]
    raffle : ProgramAccount<'info, Raffle>,
    
    #[account(mut)]
    ledger : AccountInfo<'info>,
    
    #[account(mut)]
    spot_store : AccountInfo<'info>,

    system_program : Program<'info, System>
}

#[derive(Accounts)]
pub struct TransferAuthority<'info>{
    #[account(mut)]
    owner : Signer<'info>,
    
    #[account(mut,has_one=owner)]
    raffle_system : ProgramAccount<'info, RaffleSystem>,
}

#[derive(Accounts)]
pub struct SetManager<'info>{
    #[account(mut)]
    owner : Signer<'info>,
    
    #[account(mut, constraint= owner.key()==raffle_system.manager)]
    raffle_system : ProgramAccount<'info, RaffleSystem>,
}

#[derive(Accounts)]
pub struct Pause<'info> {
    /// CHECK:
    #[account(mut, signer)]
    owner : AccountInfo<'info>,   

    /// CHECK:
    #[account(mut, constraint= owner.key()==raffle_system.manager)]
    raffle_system : ProgramAccount<'info,RaffleSystem>   
}

#[derive(Accounts)]
pub struct Claim<'info> {
    /// CHECK:
    #[account(mut, signer)]
    owner : AccountInfo<'info>,   

    /// CHECK:
    #[account(mut)]
    raffle_system : ProgramAccount<'info,RaffleSystem>,

    /// CHECK:
    #[account(mut)]
    raffle_system_address : AccountInfo<'info>
}

#[derive(Accounts)]
#[instruction(_bump : u8)]
pub struct InitRaffleSystem<'info>{
    #[account(mut)]
    owner : Signer<'info>,

    #[account(mut)]
    manager : AccountInfo<'info>,
    
    #[account(init, seeds=[(*rand.key).as_ref()], bump=_bump, payer=owner, space=8+RAFFLE_SYSTEM_SIZE)]
    raffle_system : ProgramAccount<'info, RaffleSystem>,
    
    rand : AccountInfo<'info>,

    token_mint : Account<'info, Mint>,
    
    #[account(constraint=token_account.owner==raffle_system.key()
    && token_account.mint==token_mint.key())]
    token_account : Account<'info, TokenAccount>,
    
    system_program : Program<'info, System>
}

pub const RAFFLE_SYSTEM_SIZE : usize = 32*5+1+1;
pub const MAX_ROOM_NAME_SIZE : usize = 50;
pub const LOGO_SIZE : usize = 200;
pub const DISCORD_SIZE : usize = 100;
pub const TWITTER_SIZE : usize = 100;
pub const RAFFLE_SIZE : usize = 32+MAX_ROOM_NAME_SIZE+LOGO_SIZE+DISCORD_SIZE+TWITTER_SIZE+1+8+4+4+8+8+32+32+1+4+96;
pub const SPOT_SIZE : usize = 32 + 4 + 1;
pub const USERDATA_SIZE : usize = 32+32+4+1;

#[account]
pub struct RaffleSystem{
    owner : Pubkey,
    manager : Pubkey,
    rand : Pubkey,
    pub pause_flag : bool,
    token_mint : Pubkey,
    token_account : Pubkey,
    bump : u8,
}

#[account]
pub struct Raffle{
    raffle_system : Pubkey,
    room_name : String,
    logo : String,
    discord : String,
    twitter : String,
    status : u8,
    ticket_value : u64,
    spot_num : u32,
    max_ticket_num : u32,
    start_time : u64,
    period : u64,
    ledger_account : Pubkey,
    spots_account : Pubkey,
    is_show : bool,
    max_ticket_per_user : u32,
}

#[account]
pub struct UserData{
    owner : Pubkey,
    raffle : Pubkey,
    ticket_num : u32,
    bump : u8
}

#[account]
#[derive(Default)]
pub struct Ledger{
    pub raffle_account : Pubkey,
    pub users : Vec<Pubkey>
}

pub fn get_num(
    data : &Ref<&mut [u8]>
    )-> core::result::Result<usize, ProgramError>{
    return Ok(u32::from_le_bytes(*array_ref![data,40,4]) as usize);
}

pub fn get_user(
    a : &AccountInfo,
    index : usize,
    ) -> core::result::Result<Pubkey, ProgramError> {
    let arr = a.data.borrow();
    let total = get_num(&arr)?;
    if index > total{
        return Err(PoolError::IndexGreaterThanLength.into());
    }
    let data_array = &arr[44+index*32..44+index*32+32];
    let user : Pubkey = Pubkey::try_from_slice(data_array)?;
    Ok(user)
}

pub fn set_user(
    a : &mut AccountInfo,
    index : usize,
    user : Pubkey,
    num : u32,
    ){
    let mut arr = a.data.borrow_mut();
    let data_array = user.try_to_vec().unwrap();
    let vec_start = 44+index*32;
    let length = 32 * num as usize;
    for i in 0..length{
        arr[vec_start+i] = data_array[i%32]
    }
}

pub fn set_count(
    a : &AccountInfo,
    count : u32
    ){
    let mut arr = a.data.borrow_mut();
    let data_array = count.try_to_vec().unwrap();
    let vec_start = 40;
    for i in 0..data_array.len(){
        arr[vec_start+i] = data_array[i]
    }
}

#[account]
#[derive(Default)]
pub struct SpotStore{
    pub raffle_account : Pubkey,
    pub spots : Vec<Spot>
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct Spot{
    pub nft : Pubkey,
    pub winner_ticket : u32,
    pub claimed : bool,
}

pub fn set_spot(
    a : &mut AccountInfo,
    index : usize,
    spot : Spot,
    ){
    let mut arr = a.data.borrow_mut();
    let data_array = spot.try_to_vec().unwrap();
    let vec_start = 44+index*SPOT_SIZE;
    for i in 0..SPOT_SIZE{
        arr[vec_start+i] = data_array[i];
    }
}

pub fn set_winner(
    a : &mut AccountInfo,
    winners : Vec<u32>
    ){
    let mut arr = a.data.borrow_mut();
    let mut start = 44;
    for i in 0..winners.len(){
        let data_array = winners[i].try_to_vec().unwrap();
        for i in 0..4{
            arr[start+32+i] = data_array[i];
        }
        start+=SPOT_SIZE;
    }
}

pub fn get_raffle_address(
    a : &AccountInfo
    ) -> core::result::Result<Pubkey, ProgramError> {
    let arr = a.data.borrow();
    let data_array = &arr[8..40];
    let raffle : Pubkey = Pubkey::try_from_slice(data_array)?;
    Ok(raffle)
}

pub fn get_spot(
    a : &AccountInfo,
    index : usize
    ) -> core::result::Result<Spot, ProgramError>{
    let arr = a.data.borrow();
    let data_array = &arr[44+index*SPOT_SIZE..44+index*SPOT_SIZE+SPOT_SIZE];
    let spot : Spot = Spot::try_from_slice(data_array)?;
    Ok(spot)
}

#[error]
pub enum PoolError{
    #[msg("Invalid metadata")]
    InvalidMetadata,

    #[msg("Index greater than length")]
    IndexGreaterThanLength,

    #[msg("Already ready")]
    AlreadyReady,

    #[msg("Not ready")]
    NotReady,

    #[msg("Invalid Status")]
    InvalidStatus,

    #[msg("Overflow")]
    Overflow,

    #[msg("Time out")]
    TimeOut,

    #[msg("Not match")]
    NotMatch,

    #[msg("Already claimed")]
    AlreadyClaimed,

    #[msg("Invalid index")]
    InvalidIndex,

    #[msg("Numerical overflow error")]
    NumericalOverflowError,

    #[msg("Overflow ticket num per user")]
    OverflowTicketNumPerUser,

    #[msg("Already overflow ticket num")]
    AlreadyOverflowTicketNum,

    #[msg("Invalid owner")]
    InvalidPoolOwner,

    #[msg("Insufficent Funds")]
    InsufficentFunds,

    #[msg("sol transfer failed")]
    SolTransferFailed
}
use anchor_lang::prelude::*;
use anchor_lang::system_program;
use twine_chain;

declare_id!("HBRsB8pTaY8EpR8457wmNkuFtATKzx7vxkbHSg5CRtuu");

#[derive(Clone)]
pub struct TwineChainProgram;
impl anchor_lang::Id for TwineChainProgram {
    fn id() -> Pubkey {
        twine_chain::id()
    }
}       

const DISCRIMINATOR: usize = 8;

#[program]
pub mod token_gateway {
    use super::*;

    // Initialize the Native PDA
    pub fn initialize_native_pda(ctx: Context<InitializeNativePDA>) -> Result<()> {
        let native_pda = &mut ctx.accounts.native_pda;
        native_pda.total_deposits = 0;
        Ok(())
    }

    // Handles user deposits
    pub fn deposit_sol(ctx: Context<DepositSOL>, to: Pubkey, amount: u64) -> Result<()> {
        let user = &mut ctx.accounts.user;
        let native_pda = &mut ctx.accounts.native_pda;

        // Transfer SOL from user to the NativePDA
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: user.to_account_info(),
                    to: native_pda.to_account_info(),
                },
            ),
            amount,
        )?;

        // Update total deposits in the NativePDA
        native_pda.total_deposits = native_pda
            .total_deposits
            .checked_add(amount)
            .ok_or(ErrorCode::Overflow)?;

        // call `TwineChain` to append deposit message
        let cpi_program = ctx.accounts.twine_chain_program.to_account_info();
        let cpi_accounts = twine_chain::cpi::accounts::AppendDepositMessage {
            deposit_message_pda: ctx.accounts.deposit_message_pda.to_account_info(),
            authority: user.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        let deposit_info = twine_chain::DepositMessageInfo {
            from: user.key(),
            to,
            amount,
        };

        twine_chain::cpi::append_deposit_message(cpi_ctx, deposit_info)?;

        Ok(())
    }
}

// Initialize Native PDA
#[derive(Accounts)]
pub struct InitializeNativePDA<'info> {
    #[account(
        init,
        payer = user,
        space = DISCRIMINATOR + NativePDA::INIT_SPACE,
        seeds = [b"native_pda"],
        bump
    )]
    pub native_pda: Account<'info, NativePDA>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositSOL<'info> {
    #[account(
        mut,
        seeds = [b"native_pda"],
        bump,
    )]
    pub native_pda: Account<'info, NativePDA>,

    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(
        mut,
        seeds = [b"deposit_message_pda"],
        bump
    )]
   pub deposit_message_pda: Account<'info, twine_chain::DepositMessagePDA>,
    
    pub twine_chain_program: Program<'info, TwineChainProgram>
}

// The Native PDA which holds the SOL
#[account]
#[derive(InitSpace)]
pub struct NativePDA {
    pub total_deposits: u64,
}

// Custom error codes
#[error_code]
pub enum ErrorCode {
    #[msg("The provided Bridge Account is not owned by this program.")]
    InvalidBridgeAccount,
    #[msg("Overflow occurred while updating total deposits.")]
    Overflow,
    #[msg("The deposit queue has reached its maximum capacity.")]
    QueueOverflow,
}
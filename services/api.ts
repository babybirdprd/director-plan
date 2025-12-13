import { MOCK_TICKETS, MOCK_ASSETS } from '../constants';
import { Ticket, Asset, TicketStatus } from '../types';

// Simulating API latency
const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

class MockApiService {
  private tickets: Ticket[] = [...MOCK_TICKETS];
  private assets: Asset[] = [...MOCK_ASSETS];

  async getTickets(): Promise<Ticket[]> {
    await delay(300);
    return this.tickets;
  }

  async getAssets(): Promise<Asset[]> {
    await delay(300);
    return this.assets;
  }

  async updateTicketStatus(id: string, status: TicketStatus): Promise<void> {
    await delay(200);
    this.tickets = this.tickets.map(t => 
      t.id === id ? { ...t, status } : t
    );
  }

  async verifyTicket(id: string): Promise<{ success: boolean; output: string }> {
    await delay(1500);
    return {
      success: true,
      output: "Running cargo test...\nCompiling director-engine v0.1.0\nFinished test [unoptimized + debuginfo] target(s) in 0.45s\nRunning tests...\ntest tests::verify_visual_regression ... ok\n\ntest result: ok. 1 passed; 0 failed;"
    };
  }

  async uploadAsset(file: File): Promise<Asset> {
    await delay(800);
    const newAsset: Asset = {
      id: `A-${Date.now()}`,
      name: file.name,
      type: file.type.includes('image') ? 'image' : file.name.endsWith('json') ? 'lottie' : 'font',
      path: `assets/uploads/${file.name}`,
      rust_id: `ASSET_${file.name.toUpperCase().replace(/[^A-Z0-9]/g, '_')}`,
      preview_url: file.type.includes('image') ? URL.createObjectURL(file) : undefined
    };
    this.assets.push(newAsset);
    return newAsset;
  }
}

export const api = new MockApiService();
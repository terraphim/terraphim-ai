import { render, screen, waitFor } from '@testing-library/svelte';
import NodesView from './NodesView.svelte';
import { role } from '../stores';
import { get } from 'svelte/store';

// Mock fetch
global.fetch = jest.fn();

describe('NodesView', () => {
    beforeEach(() => {
        (global.fetch as jest.Mock).mockClear();
    });

    it('should show loading state initially', () => {
        render(NodesView);
        expect(screen.getByText('Loading nodes...')).toBeInTheDocument();
    });

    it('should display nodes when data is loaded', async () => {
        const mockNodes = {
            status: "success",
            nodes: [
                {
                    id: "1",
                    normalized_term: "test term 1",
                    total_documents: 5,
                    ranks: [
                        { edge_weight: 10, document_id: "doc1" },
                        { edge_weight: 8, document_id: "doc2" }
                    ]
                },
                {
                    id: "2",
                    normalized_term: "test term 2",
                    total_documents: 3,
                    ranks: [
                        { edge_weight: 6, document_id: "doc3" }
                    ]
                }
            ]
        };

        (global.fetch as jest.Mock).mockImplementationOnce(() =>
            Promise.resolve({
                ok: true,
                json: () => Promise.resolve(mockNodes)
            })
        );

        render(NodesView);

        await waitFor(() => {
            // Check if IcicleChart is rendered with correct data
            const chartContainer = screen.getByTestId('chart-container');
            expect(chartContainer).toBeInTheDocument();
        });
    });

    it('should show error message when API call fails', async () => {
        (global.fetch as jest.Mock).mockImplementationOnce(() =>
            Promise.reject(new Error('API Error'))
        );

        render(NodesView);

        await waitFor(() => {
            expect(screen.getByText(/Error fetching nodes/)).toBeInTheDocument();
        });
    });

    it('should refetch data when role changes', async () => {
        const mockNodes = {
            status: "success",
            nodes: []
        };

        (global.fetch as jest.Mock).mockImplementation(() =>
            Promise.resolve({
                ok: true,
                json: () => Promise.resolve(mockNodes)
            })
        );

        render(NodesView);
        
        // Initial fetch
        expect(global.fetch).toHaveBeenCalledTimes(1);

        // Change role
        role.set('new role');
        
        await waitFor(() => {
            // Should fetch again with new role
            expect(global.fetch).toHaveBeenCalledTimes(2);
            expect(global.fetch).toHaveBeenLastCalledWith(
                expect.any(String),
                expect.objectContaining({
                    body: JSON.stringify({ role: 'new role' })
                })
            );
        });
    });
}); 
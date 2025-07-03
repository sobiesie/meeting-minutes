"use client"

import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import * as z from 'zod';
import { FormInputItem } from "./molecules/form-components/form-input-item";
import { Form } from "./ui/form";
import { Button } from "./ui/button";


const serverSettingsSchema = z.object({
    appServerUrl: z.string().min(1, { message: "App server URL is required" }),
    transcriptServerUrl: z.string().min(1, { message: "Transcript server URL is required" }),
});

type ServerSettings = z.infer<typeof serverSettingsSchema>;

export function ServerSettings() {
    const form = useForm<ServerSettings>({
        resolver: zodResolver(serverSettingsSchema),
        defaultValues: {
            appServerUrl: '',
            transcriptServerUrl: '',
        },
    });
    const onSubmit = (data: ServerSettings) => {
        console.log(data);
    };
    return (
        <Form {...form}>
            <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-8">
                <h3 className="text-lg font-semibold text-gray-900">Server Settings</h3>
                <FormInputItem 
                    name="appServerUrl"
                    control={form.control}
                    label="App Server URL"
                    type="text"
                    placeholder="Enter app server URL"
                />
                <FormInputItem 
                    name="transcriptServerUrl"
                    control={form.control}
                    label="Transcript Server URL"
                    type="text"
                    placeholder="Enter transcript server URL"
                />
                <Button type="submit">Save</Button>
            </form>
        </Form>
    );
}
